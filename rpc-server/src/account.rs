use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, bail, ensure, Result};
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::Error;
use async_tungstenite::tungstenite::Message;
use broadcaster::BroadcastChannel;
use deltachat::{
    chat::{self, Chat, ChatId},
    chatlist::Chatlist,
    contact::Contact,
    context::Context,
    message::{self, MsgId},
    Event,
};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use num_traits::{FromPrimitive, ToPrimitive};
use serde::Serialize;
use shared::{ChatMessage, ChatState, Login, Viewtype};

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
    pub chat_states: HashMap<ChatId, ChatState>,
    pub selected_chat: Option<ChatState>,
    pub selected_chat_id: Option<ChatId>,
    pub chatlist: Chatlist,
    /// Messages of the selected chat
    pub chat_msg_ids: Vec<MsgId>,
    /// State of currently selected chat messages
    pub chat_msgs: HashMap<String, ChatMessage>,
    /// indexed by index in the Chatlist
    pub chats: HashMap<ChatId, Chat>,
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

        let context = Context::new("desktop".into(), path.into())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        let chatlist = Chatlist::try_load(&context, 0, None, None)
            .await
            .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

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
                chats: Default::default(),
                selected_chat: None,
                selected_chat_id: None,
                chatlist,
                chat_msg_ids: Default::default(),
                chat_msgs: Default::default(),
                chat_states: Default::default(),
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

        imex::imex(&self.context, imex::ImexMode::ImportBackup, Some(path))
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let mut events = self.events.clone();
        while let Some(event) = events.next().await {
            match event {
                Event::ImexProgress(0) => {
                    bail!("Failed to import");
                }
                Event::ImexProgress(1000) => {
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
        info!("configure");

        self.context
            .configure()
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        let mut events = self.events.clone();
        while let Some(event) = events.next().await {
            info!("configure event {:?}", event);
            match event {
                Event::ConfigureProgress(0) => {
                    bail!("Failed to login");
                }
                Event::ConfigureProgress(1000) => {
                    break;
                }
                Event::ImapConnected(_) | Event::SmtpConnected(_) => {
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn load_chat_list(&self, start_index: usize, stop_index: usize) -> Result<()> {
        ensure!(start_index <= stop_index, "invalid indicies");
        self.state.write().await.chat_states.clear();

        let ids = {
            let chatlist = &self.state.read().await.chatlist;
            (start_index..=stop_index)
                .map(|i| chatlist.get_chat_id(i))
                .collect::<Vec<_>>()
        };

        let futures = ids
            .into_iter()
            .map(|chat_id| refresh_chat_state(self.context.clone(), self.state.clone(), chat_id))
            .collect::<Vec<_>>();

        futures::future::try_join_all(futures).await?;

        Ok(())
    }

    pub async fn select_chat(&self, chat_id: ChatId) -> Result<()> {
        info!("selecting chat {:?}", chat_id);
        let (chat, chat_state) =
            load_chat_state(self.context.clone(), self.state.clone(), chat_id).await?;

        let mut ls = self.state.write().await;
        ls.selected_chat_id = Some(chat_id);
        ls.chat_msg_ids = chat::get_chat_msgs(&self.context, chat_id, 0, None)
            .await
            .into_iter()
            .filter_map(|c| match c {
                deltachat::chat::ChatItem::Message { msg_id } => Some(msg_id),
                _ => None,
            })
            .collect();
        ls.chat_msgs = Default::default();

        // mark as noticed
        chat::marknoticed_chat(&self.context, chat_id)
            .await
            .map_err(|err| anyhow!("failed to mark noticed: {:?}", err))?;

        if let Some(chat_state) = chat_state {
            ls.selected_chat = Some(chat_state);
        }

        ls.chats.insert(chat.id, chat);

        Ok(())
    }

    pub async fn load_message_list(&self) -> Result<()> {
        refresh_message_list(self.context.clone(), self.state.clone(), None).await?;

        // markseen messages that we load
        // could be better, by checking actual in view, but close enough for now
        let msgs_list = self.state.read().await.chat_msg_ids.clone();
        message::markseen_msgs(&self.context, msgs_list).await;

        Ok(())
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
        let context = self.context.clone();

        task::spawn(async move {
            // subscribe to events
            while let Some(event) = events.next().await {
                let res = match event {
                    Event::ConfigureProgress(0) => {
                        state.write().await.logged_in = Login::Error("failed to login".into());
                        let local_state = local_state.read().await;
                        local_state.send_update(writer.clone()).await
                    }
                    Event::ImexProgress(0) => {
                        state.write().await.logged_in = Login::Error("failed to import".into());
                        let local_state = local_state.read().await;
                        local_state.send_update(writer.clone()).await
                    }
                    Event::ConfigureProgress(1000)
                    | Event::ImexProgress(1000)
                    | Event::ImapConnected(_)
                    | Event::SmtpConnected(_) => {
                        info!("logged in");
                        state.write().await.logged_in = Login::Success;
                        let local_state = local_state.read().await;
                        local_state.send_update(writer.clone()).await
                    }
                    Event::ConfigureProgress(i) | Event::ImexProgress(i) => {
                        info!("configure progres: {}/1000", i);
                        state.write().await.logged_in = Login::Progress(i);
                        let local_state = local_state.read().await;
                        local_state.send_update(writer.clone()).await
                    }
                    Event::MsgsChanged { chat_id, .. }
                    | Event::IncomingMsg { chat_id, .. }
                    | Event::MsgDelivered { chat_id, .. }
                    | Event::MsgRead { chat_id, .. }
                    | Event::MsgFailed { chat_id, .. }
                    | Event::ChatModified(chat_id) => {
                        let res =
                            refresh_message_list(context.clone(), state.clone(), Some(chat_id))
                                .try_join(refresh_chat_list(context.clone(), state.clone()))
                                .try_join(refresh_chat_state(
                                    context.clone(),
                                    state.clone(),
                                    chat_id,
                                ))
                                .await;

                        if let Err(err) = res {
                            Err(err)
                        } else {
                            let local_state = local_state.read().await;
                            local_state.send_update(writer.clone()).await
                        }
                    }
                    Event::Info(msg) => {
                        info!("{}", msg);
                        Ok(())
                    }
                    Event::Warning(msg) => {
                        warn!("{}", msg);
                        Ok(())
                    }
                    Event::Error(msg) => {
                        error!("{}", msg);
                        Ok(())
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

pub async fn refresh_chat_state(
    context: Context,
    state: Arc<RwLock<AccountState>>,
    chat_id: ChatId,
) -> Result<()> {
    info!("refreshing chat state: {:?}", &chat_id);

    let (chat, chat_state) = load_chat_state(context, state.clone(), chat_id).await?;

    let mut state = state.write().await;
    if let Some(chat_state) = chat_state {
        if let Some(sel_chat_id) = state.selected_chat_id {
            if sel_chat_id == chat_id {
                state.selected_chat = Some(chat_state.clone());
            }
        }

        if chat_state.index.is_some() {
            // Only insert if there is actually a valid index.
            state.chat_states.insert(chat_id, chat_state);
        }
    }
    state.chats.insert(chat.id, chat);

    Ok(())
}

async fn load_chat_state(
    context: Context,
    state: Arc<RwLock<AccountState>>,
    chat_id: ChatId,
) -> Result<(Chat, Option<ChatState>)> {
    let state = state.read().await;
    let chats = &state.chatlist;
    let chat = Chat::load_from_db(&context, chat_id)
        .await
        .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

    let chat_state = if let Some(index) = chats.get_index_for_id(chat_id) {
        let lot = chats.get_summary(&context, index, Some(&chat)).await;

        let header = lot.get_text1().map(|s| s.to_string()).unwrap_or_default();
        let preview = lot.get_text2().map(|s| s.to_string()).unwrap_or_default();

        let index = state.chatlist.get_index_for_id(chat_id);

        Some(ChatState {
            id: chat_id.to_u32(),
            index,
            name: chat.get_name().to_string(),
            header,
            preview,
            timestamp: lot.get_timestamp(),
            state: lot.get_state().to_string(),
            profile_image: chat.get_profile_image(&context).await.map(Into::into),
            can_send: chat.can_send(),
            chat_type: chat.get_type().to_string(),
            color: chat.get_color(&context).await,
            is_device_talk: chat.is_device_talk(),
            is_self_talk: chat.is_self_talk(),
            fresh_msg_cnt: chat_id.get_fresh_msg_cnt(&context).await,
        })
    } else {
        None
    };

    Ok((chat, chat_state))
}

pub async fn refresh_chat_list(context: Context, state: Arc<RwLock<AccountState>>) -> Result<()> {
    let chatlist = Chatlist::try_load(&context, 0, None, None)
        .await
        .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

    state.write().await.chatlist = chatlist;

    Ok(())
}

pub async fn refresh_message_list(
    context: Context,
    state: Arc<RwLock<AccountState>>,
    chat_id: Option<ChatId>,
) -> Result<()> {
    let mut ls = state.write().await;
    let current_chat_id = ls.selected_chat_id.clone();
    if chat_id.is_some() && current_chat_id != chat_id {
        return Ok(());
    }
    if current_chat_id.is_none() {
        // Ignore if no chat is selected
        return Ok(());
    }

    info!("loading chat messages {:?}", chat_id);

    ls.chat_msg_ids = chat::get_chat_msgs(&context, current_chat_id.unwrap(), 0, None)
        .await
        .into_iter()
        .filter_map(|c| match c {
            deltachat::chat::ChatItem::Message { msg_id } => Some(msg_id),
            _ => None,
        })
        .collect();

    let mut msgs = HashMap::with_capacity(ls.chat_msg_ids.len());
    for (i, msg_id) in ls.chat_msg_ids.iter().enumerate() {
        let msg = message::Message::load_from_db(&context, *msg_id)
            .await
            .map_err(|err| anyhow!("failed to load msg: {}: {}", msg_id, err))?;

        let from = Contact::load_from_db(&context, msg.get_from_id())
            .await
            .map_err(|err| anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err))?;

        let chat_msg = ChatMessage {
            id: msg.get_id().to_u32(),
            from_id: msg.get_from_id(),
            viewtype: Viewtype::from_i32(msg.get_viewtype().to_i32().unwrap()).unwrap(),
            from_first_name: from.get_first_name().to_string(),
            from_profile_image: from.get_profile_image(&context).await.map(Into::into),
            from_color: from.get_color(),
            starred: msg.is_starred(),
            state: msg.get_state().to_string(),
            text: msg.get_text(),
            timestamp: msg.get_sort_timestamp(),
            is_info: msg.is_info(),
            file: msg.get_file(&context).map(Into::into),
            file_width: msg.get_width(),
            file_height: msg.get_height(),
        };
        msgs.insert(i.to_string(), chat_msg);
    }

    ls.chat_msgs = msgs;

    Ok(())
}
