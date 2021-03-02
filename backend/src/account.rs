use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, ensure, Result};
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::Error;
use async_tungstenite::tungstenite::Message;
use broadcaster::BroadcastChannel;
use chrono::prelude::*;
use deltachat::{
    chat::{self, Chat, ChatId},
    chatlist::Chatlist,
    contact::Contact,
    context::Context,
    message::{self, MsgId},
    Event, EventType,
};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use num_traits::{FromPrimitive, ToPrimitive};
use serde::Serialize;
use shared::{ChatItem, ChatMessage, ChatState, InnerChatMessage, Login, Viewtype};

use crate::state::*;

lazy_static! {
    pub static ref HOME_DIR: PathBuf = dirs::home_dir()
        .unwrap_or_else(|| "home".into())
        .join(".deltachat");
}

#[derive(Debug)]
pub struct Account {
    pub context: Context,
    pub state: Arc<RwLock<AccountState>>,
    pub events: BroadcastChannel<Event>,
}

impl Drop for Account {
    fn drop(&mut self) {
        task::block_on(self.context.stop_io());
    }
}

#[derive(Debug)]
pub struct AccountState {
    pub logged_in: Login,
    pub email: String,
    pub selected_chat_id: Option<ChatId>,
    pub selected_chat: Option<ChatState>,
}

impl Account {
    pub async fn new(email: &str) -> Result<Self> {
        let receiver = BroadcastChannel::new();

        // TODO: escape email to be a vaild filesystem name
        let path = HOME_DIR.join(format!("{}.sqlite", email));

        // Ensure the folders actually exist
        if let Some(parent) = path.parent() {
            async_std::fs::create_dir_all(parent).await?;
        }

        let context = Context::new("desktop".into(), path.into(), 0)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        let sender = receiver.clone();
        let events = context.get_event_emitter();

        task::spawn(async move {
            while let Some(event) = events.recv().await {
                if let Err(err) = sender.send(&event).await {
                    error!("Failed to send event: {:?}", err);
                }
            }
        });

        let account = Account {
            context,
            state: Arc::new(RwLock::new(AccountState {
                logged_in: Login::default(),
                email: email.to_string(),
                selected_chat_id: None,
                selected_chat: None,
            })),
            events: receiver,
        };

        Ok(account)
    }

    pub async fn logged_in(&self) -> bool {
        self.state.read().await.logged_in == Login::Success
    }

    pub async fn import(&self, path: &str) -> Result<()> {
        use deltachat::imex;

        imex::imex(&self.context, imex::ImexMode::ImportBackup, path)
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let mut events = self.events.clone();
        while let Some(event) = events.next().await {
            match event.typ {
                EventType::ImexProgress(0) => {
                    bail!("Failed to import");
                }
                EventType::ImexProgress(1000) => {
                    self.context.start_io().await;
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<()> {
        use deltachat::config::Config;
        self.state.write().await.logged_in = Login::Progress(0);

        self.context
            .set_config(Config::Addr, Some(email))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        self.context
            .set_config(Config::MailPw, Some(password))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        self.configure().await?;
        Ok(())
    }

    pub async fn configure(&self) -> Result<()> {
        use deltachat::config::Config;
        info!("configure");

        let is_configured = self.context.get_config_int(Config::Configured).await;
        if is_configured == 1 {
            info!("Account already configured");
            return Ok(());
        } else {
            self.context
                .configure()
                .await
                .map_err(|err| anyhow!("{:?}", err))?;

            let mut events = self.events.clone();
            while let Some(event) = events.next().await {
                info!("configure event {:?}", event);
                match event.typ {
                    EventType::ConfigureProgress { progress, .. } => match progress {
                        0 => {
                            bail!("Failed to login");
                        }
                        1000 => {
                            break;
                        }
                        _ => {}
                    },
                    EventType::ImapConnected(_) | EventType::SmtpConnected(_) => {
                        break;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }

    pub async fn load_chat_list(
        &self,
        start_index: usize,
        stop_index: usize,
    ) -> Result<((usize, usize), usize, Vec<ChatState>)> {
        ensure!(start_index <= stop_index, "invalid indicies");

        let chatlist = Chatlist::try_load(&self.context, 0, None, None)
            .await
            .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

        let total_len = chatlist.len();
        let len = stop_index.saturating_sub(start_index);

        let mut chats = Vec::with_capacity(len);
        for i in start_index..=stop_index {
            let chat_id = chatlist.get_chat_id(i);
            let (_, chat_state) = load_chat_state(self.context.clone(), chat_id).await?;
            if let Some(s) = chat_state {
                chats.push(s);
            }
        }

        Ok(((start_index, stop_index), total_len, chats))
    }

    pub async fn select_chat(&self, chat_id: ChatId) -> Result<()> {
        info!("selecting chat {:?}", chat_id);
        let mut ls = self.state.write().await;
        ls.selected_chat_id = Some(chat_id);
        let (_, selected_chat) = load_chat_state(self.context.clone(), chat_id).await?;
        ls.selected_chat = selected_chat;

        // mark as noticed
        chat::marknoticed_chat(&self.context, chat_id)
            .await
            .map_err(|err| anyhow!("failed to mark noticed: {:?}", err))?;

        Ok(())
    }

    pub async fn load_message_list(
        &self,
        range: Option<(usize, usize)>,
    ) -> Result<(u32, (usize, usize), Vec<ChatItem>, Vec<ChatMessage>)> {
        let chat_id = self.state.read().await.selected_chat_id.clone();
        if let Some(chat_id) = chat_id {
            info!("loading {:?} msgs", chat_id);

            let (chat_id, range, chat_items, chat_messages) =
                refresh_message_list(self.context.clone(), chat_id, range).await?;

            let msg_ids = chat_messages
                .iter()
                .filter_map(|item| match item {
                    ChatMessage::Message(inner) => Some(message::MsgId::new(inner.id)),
                    ChatMessage::DayMarker(..) => None,
                })
                .collect();
            message::markseen_msgs(&self.context, msg_ids).await;

            Ok((chat_id, range, chat_items, chat_messages))
        } else {
            bail!("failed to load message list, no chat selected");
        }
    }

    pub async fn send_text_message(&self, text: String) -> Result<()> {
        if let Some(chat_id) = self.state.read().await.selected_chat_id {
            chat::send_text_msg(&self.context, chat_id, text)
                .await
                .map_err(|err| anyhow!("failed to send message: {}", err))?;
        } else {
            bail!("no chat selected, can not send message");
        }

        Ok(())
    }

    pub async fn send_file_message(
        &self,
        typ: Viewtype,
        path: String,
        text: Option<String>,
        mime: Option<String>,
    ) -> Result<()> {
        if let Some(chat_id) = self.state.read().await.selected_chat_id {
            let mut msg = message::Message::new(
                deltachat::constants::Viewtype::from_i32(typ.to_i32().unwrap()).unwrap(),
            );
            msg.set_text(text);
            msg.set_file(path, mime.as_deref());

            chat::send_msg(&self.context, chat_id, &mut msg)
                .await
                .map_err(|err| anyhow!("failed to send message: {}", err))?;
        } else {
            bail!("no chat selected, can not send message");
        }

        Ok(())
    }

    pub async fn create_chat_by_id(&self, id: MsgId) -> Result<ChatId> {
        let chat = chat::create_by_msg_id(&self.context, id)
            .await
            .map_err(|err| anyhow!("failed to create chat: {}", err))?;

        // TODO: select that chat?
        Ok(chat)
    }

    pub async fn maybe_network(&self) {
        self.context.maybe_network().await;
    }

    pub fn subscribe<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
        local_state: Arc<RwLock<LocalState>>,
    ) where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        info!("Subscribed");
        let mut events = self.events.clone();
        let state = self.state.clone();
        let ctx = self.context.clone();

        task::spawn(async move {
            let ctx = &ctx;
            // subscribe to events
            while let Some(event) = events.next().await {
                let res = match event.typ {
                    EventType::ConfigureProgress { progress, .. } => {
                        if progress == 0 {
                            state.write().await.logged_in = Login::Error("failed to login".into());
                            let local_state = local_state.read().await;
                            local_state
                                .send_event(
                                    writer.clone(),
                                    event.id,
                                    shared::Event::Configure(shared::Progress::Error),
                                )
                                .await
                        } else {
                            let p = if progress == 1000 {
                                shared::Progress::Success
                            } else {
                                state.write().await.logged_in = Login::Progress(progress);
                                shared::Progress::Step(progress)
                            };
                            local_state
                                .read()
                                .await
                                .send_event(writer.clone(), event.id, shared::Event::Configure(p))
                                .await
                        }
                    }
                    EventType::ImexProgress(progress) => {
                        if progress == 0 {
                            state.write().await.logged_in = Login::Error("failed to import".into());
                            let local_state = local_state.read().await;
                            local_state
                                .send_event(
                                    writer.clone(),
                                    event.id,
                                    shared::Event::Imex(shared::Progress::Error),
                                )
                                .await
                        } else {
                            let p = if progress == 1000 {
                                shared::Progress::Success
                            } else {
                                state.write().await.logged_in = Login::Progress(progress);
                                shared::Progress::Step(progress)
                            };
                            local_state
                                .read()
                                .await
                                .send_event(writer.clone(), event.id, shared::Event::Imex(p))
                                .await
                        }
                    }
                    EventType::ImapConnected(_) | EventType::SmtpConnected(_) => {
                        info!("logged in");
                        state.write().await.logged_in = Login::Success;
                        local_state
                            .read()
                            .await
                            .send_event(writer.clone(), event.id, shared::Event::Connected)
                            .await
                    }
                    EventType::IncomingMsg { chat_id, msg_id } => {
                        let load = || async {
                            let msg = message::Message::load_from_db(ctx, msg_id).await.map_err(
                                |err| anyhow!("failed to load msg: {}: {}", msg_id, err),
                            )?;
                            let chat = Chat::load_from_db(ctx, chat_id)
                                .await
                                .map_err(|err| anyhow!("failed to load chat: {:?}", err))?;

                            local_state
                                .read()
                                .await
                                .send_event(
                                    writer.clone(),
                                    event.id,
                                    shared::Event::MessageIncoming {
                                        chat_id: chat_id.to_u32(),
                                        title: chat.get_name().to_string(),
                                        body: msg.get_text().unwrap_or_default(),
                                    },
                                )
                                .await
                        };
                        load().await
                    }
                    EventType::MsgDelivered { chat_id, .. }
                    | EventType::MsgFailed { chat_id, .. }
                    | EventType::MsgsChanged { chat_id, .. }
                    | EventType::MsgRead { chat_id, .. }
                    | EventType::ChatModified(chat_id)
                    | EventType::MsgsNoticed(chat_id) => {
                        local_state
                            .read()
                            .await
                            .send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::MessagesChanged {
                                    chat_id: chat_id.to_u32(),
                                },
                            )
                            .await
                    }
                    EventType::Info(msg) => {
                        info!("{}", msg);
                        local_state
                            .read()
                            .await
                            .send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::Log(shared::Log::Info(msg)),
                            )
                            .await
                    }
                    EventType::Warning(msg) => {
                        warn!("{}", msg);
                        local_state
                            .read()
                            .await
                            .send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::Log(shared::Log::Warning(msg)),
                            )
                            .await
                    }
                    EventType::Error(msg) => {
                        error!("{}", msg);
                        local_state
                            .read()
                            .await
                            .send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::Log(shared::Log::Error(msg)),
                            )
                            .await
                    }
                    _ => {
                        debug!("{:?}", event);
                        Ok(())
                    }
                };

                match res {
                    Ok(_) => {}
                    Err(err) => match err.downcast_ref::<Error>() {
                        Some(Error::ConnectionClosed) => {
                            // stop listening
                            break;
                        }
                        _ => {}
                    },
                }
            }
        });
    }
}

#[derive(Debug, Serialize)]
pub struct RemoteEvent {
    #[serde(rename = "type")]
    typ: String,
    event: String,
}

fn get_timestamp(ts: i64) -> DateTime<Utc> {
    let naive = NaiveDateTime::from_timestamp(ts, 0);
    DateTime::from_utc(naive, Utc)
}

async fn load_chat_state(context: Context, chat_id: ChatId) -> Result<(Chat, Option<ChatState>)> {
    let chats = Chatlist::try_load(&context, 0, None, None)
        .await
        .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

    let chat = Chat::load_from_db(&context, chat_id)
        .await
        .map_err(|err| anyhow!("failed to load chat: {:?}", err))?;

    let chat_state = if let Some(index) = chats.get_index_for_id(chat_id) {
        let lot = chats.get_summary(&context, index, Some(&chat)).await;

        let header = lot.get_text1().map(|s| s.to_string()).unwrap_or_default();
        let preview = lot.get_text2().map(|s| s.to_string()).unwrap_or_default();

        let index = chats.get_index_for_id(chat_id);

        Some(ChatState {
            id: chat_id.to_u32(),
            index,
            name: chat.get_name().to_string(),
            header,
            preview,
            timestamp: get_timestamp(lot.get_timestamp()),
            state: lot.get_state().to_string(),
            profile_image: chat.get_profile_image(&context).await.map(Into::into),
            can_send: chat.can_send(),
            chat_type: chat.get_type().to_string(),
            color: chat.get_color(&context).await,
            is_device_talk: chat.is_device_talk(),
            is_self_talk: chat.is_self_talk(),
            fresh_msg_cnt: chat_id.get_fresh_msg_cnt(&context).await,
            member_count: deltachat::chat::get_chat_contacts(&context, chat_id)
                .await
                .len(),
        })
    } else {
        None
    };

    Ok((chat, chat_state))
}

async fn refresh_message_list(
    context: Context,
    chat_id: ChatId,
    range: Option<(usize, usize)>,
) -> Result<(u32, (usize, usize), Vec<ChatItem>, Vec<ChatMessage>)> {
    let chat_items: Vec<_> = chat::get_chat_msgs(
        &context,
        chat_id,
        deltachat::constants::DC_GCM_ADDDAYMARKER,
        None,
    )
    .await
    .into_iter()
    .filter_map(|item| match item {
        chat::ChatItem::Message { msg_id } => Some(ChatItem::Message(msg_id.to_u32())),
        chat::ChatItem::DayMarker { timestamp } => {
            Some(ChatItem::DayMarker(get_timestamp(timestamp * 86_400)))
        }
        _ => None,
    })
    .collect();

    let total_len = chat_items.len();

    // default to the last n items
    let range = range.unwrap_or_else(|| (total_len.saturating_sub(40), total_len));

    info!(
        "loading chat messages {:?} from ({}..={})",
        chat_id, range.0, range.1
    );

    let len = range.1.saturating_sub(range.0);
    let offset = range.0;

    let mut chat_messages = Vec::with_capacity(len);
    let mut contacts = HashMap::new();

    let mut last_contact_id = None;
    let mut last_marker = true;
    for chat_item in chat_items.iter().skip(offset).take(len) {
        match chat_item {
            ChatItem::Message(msg_id) => {
                let msg = message::Message::load_from_db(&context, MsgId::new(*msg_id))
                    .await
                    .map_err(|err| anyhow!("failed to load msg: {}: {}", msg_id, err))?;

                let from = match contacts.get(&msg.get_from_id()) {
                    Some(contact) => contact,
                    None => {
                        let contact = Contact::load_from_db(&context, msg.get_from_id())
                            .await
                            .map_err(|err| {
                                anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err)
                            })?;
                        contacts.insert(msg.get_from_id(), contact);
                        contacts.get(&msg.get_from_id()).unwrap()
                    }
                };

                let is_first = if last_marker {
                    true
                } else {
                    if let Some(id) = last_contact_id {
                        id != msg.get_from_id()
                    } else {
                        true
                    }
                };
                last_contact_id = Some(msg.get_from_id());
                last_marker = false;
                let mut inner_msg = InnerChatMessage {
                    id: msg.get_id().to_u32(),
                    from_id: msg.get_from_id(),
                    viewtype: Viewtype::from_i32(msg.get_viewtype().to_i32().unwrap()).unwrap(),
                    from_first_name: from.get_display_name().to_string(),
                    from_profile_image: from.get_profile_image(&context).await.map(Into::into),
                    from_color: from.get_color(),
                    state: msg.get_state().to_string(),
                    text: msg.get_text(),
                    quote: None,
                    timestamp: get_timestamp(msg.get_sort_timestamp()),
                    is_info: msg.is_info(),
                    file: msg.get_file(&context).map(Into::into),
                    file_width: msg.get_width(),
                    file_height: msg.get_height(),
                    is_first,
                };

                if let Some(quote) = msg.quoted_message(&context).await? {
                    inner_msg.quote = Some(load_quote(&context, &mut contacts, quote).await?);
                }

                chat_messages.push(ChatMessage::Message(inner_msg));
            }
            ChatItem::DayMarker(t) => {
                chat_messages.push(ChatMessage::DayMarker(*t));
                last_marker = true;
            }
        }
    }

    Ok((chat_id.to_u32(), range, chat_items, chat_messages))
}

async fn load_quote(
    context: &Context,
    contacts: &mut HashMap<u32, Contact>,
    msg: message::Message,
) -> Result<Box<InnerChatMessage>> {
    let from = match contacts.get(&msg.get_from_id()) {
        Some(contact) => contact,
        None => {
            let contact = Contact::load_from_db(&context, msg.get_from_id())
                .await
                .map_err(|err| anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err))?;
            contacts.insert(msg.get_from_id(), contact);
            contacts.get(&msg.get_from_id()).unwrap()
        }
    };

    Ok(Box::new(InnerChatMessage {
        id: msg.get_id().to_u32(),
        from_id: msg.get_from_id(),
        viewtype: Viewtype::from_i32(msg.get_viewtype().to_i32().unwrap()).unwrap(),
        from_first_name: from.get_display_name().to_string(),
        from_profile_image: from.get_profile_image(&context).await.map(Into::into),
        from_color: from.get_color(),
        state: msg.get_state().to_string(),
        text: msg.get_text(),
        quote: None,
        timestamp: get_timestamp(msg.get_sort_timestamp()),
        is_info: msg.is_info(),
        file: msg.get_file(&context).map(Into::into),
        file_width: msg.get_width(),
        file_height: msg.get_height(),
        is_first: true,
    }))
}
