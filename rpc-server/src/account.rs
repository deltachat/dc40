use async_std::prelude::*;

use anyhow::{anyhow, bail, ensure, Result};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::Error;
use async_tungstenite::tungstenite::Message;
use broadcaster::BroadcastChannel;
use deltachat::{
    chat::{self, Chat, ChatId},
    chatlist::Chatlist,
    constants::{Chattype, Viewtype},
    contact::Contact,
    context::Context,
    message::{self, MessageState, MsgId},
    Event,
};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::state::*;

lazy_static! {
    pub static ref HOME_DIR: PathBuf = dirs::home_dir()
        .unwrap_or_else(|| "home".into())
        .join(".deltachat");
}

#[derive(Debug)]
pub struct Account {
    pub context: Arc<Context>,
    pub state: Arc<RwLock<AccountState>>,
    pub running: Arc<AtomicBool>,
    pub events: BroadcastChannel<Event>,
    imap_handle: Option<std::thread::JoinHandle<()>>,
    mvbox_handle: Option<std::thread::JoinHandle<()>>,
    sentbox_handle: Option<std::thread::JoinHandle<()>>,
    smtp_handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug)]
pub struct AccountState {
    pub logged_in: Login,
    pub email: String,
    pub chat_states: HashMap<usize, ChatState>,
    pub selected_chat: Option<ChatState>,
    pub selected_chat_id: Option<ChatId>,
    pub chatlist: Chatlist,
    /// Messages of the selected chat
    pub chat_msg_ids: Vec<MsgId>,
    /// State of currently selected chat messages
    pub chat_msgs: HashMap<usize, ChatMessage>,
    chat_msgs_range: (usize, usize),
    /// indexed by index in the Chatlist
    pub chats: HashMap<ChatId, Chat>,
}

#[derive(Default, Debug, Serialize, Clone, Deserialize)]
pub struct ChatMessage {
    id: MsgId,
    from_id: u32,
    from_first_name: String,
    from_profile_image: Option<PathBuf>,
    from_color: u32,
    viewtype: Viewtype,
    state: MessageState,
    text: Option<String>,
    starred: bool,
    timestamp: i64,
    is_info: bool,
    file: Option<PathBuf>,
    file_height: i32,
    file_width: i32,
}

#[derive(Default, Debug, Serialize, Clone, Deserialize)]
pub struct ChatState {
    id: ChatId,
    name: String,
    subtitle: String,
    header: String,
    preview: String,
    timestamp: i64,
    state: String,
    profile_image: Option<PathBuf>,
    fresh_msg_cnt: usize,
    can_send: bool,
    is_self_talk: bool,
    is_device_talk: bool,
    chat_type: Chattype,
    color: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Login {
    Success,
    Error(String),
    Progress(usize),
    Not,
}

impl Drop for Account {
    fn drop(&mut self) {
        use deltachat::job::*;

        self.running.store(false, Ordering::Relaxed);

        interrupt_inbox_idle(&self.context);
        interrupt_mvbox_idle(&self.context);
        interrupt_sentbox_idle(&self.context);
        interrupt_smtp_idle(&self.context);

        if let Some(handle) = self.imap_handle.take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.mvbox_handle.take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.sentbox_handle.take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.smtp_handle.take() {
            handle.join().unwrap();
        }
    }
}

impl Default for Login {
    fn default() -> Self {
        Login::Not
    }
}

macro_rules! while_running {
    ($running:expr, $code:block) => {
        if $running.load(Ordering::Relaxed) {
            $code
        } else {
            break;
        }
    };
}

impl Account {
    pub async fn new(email: &str) -> Result<Self> {
        let receiver = BroadcastChannel::new();
        let sender = receiver.clone();

        // TODO: escape email to be a vaild filesystem name
        let path = HOME_DIR.join(format!("{}.sqlite", email));

        // Ensure the folders actually exist
        if let Some(parent) = path.parent() {
            async_std::fs::create_dir_all(parent).await?;
        }

        let context = task::spawn_blocking(move || {
            Context::new(
                Box::new(move |_ctx, event| {
                    if let Err(err) = task::block_on(sender.send(&event)) {
                        warn!("failed to send: {:?}", err);
                    }
                }),
                "desktop".into(),
                path.into(),
            )
            .map_err(|err| anyhow!("{:?}", err))
        })
        .await?;

        let context = Arc::new(context);
        let ctx = context.clone();
        let chatlist = task::spawn_blocking(move || {
            Chatlist::try_load(&ctx, 0, None, None)
                .map_err(|err| anyhow!("failed to load chats: {:?}", err))
        })
        .await?;

        let mut account = Account {
            context,
            state: Arc::new(RwLock::new(AccountState {
                logged_in: Login::default(),
                email: email.to_string(),
                chats: Default::default(),
                selected_chat: None,
                selected_chat_id: None,
                chatlist,
                chat_msgs_range: (0, 0),
                chat_msg_ids: Default::default(),
                chat_msgs: Default::default(),
                chat_states: Default::default(),
            })),
            imap_handle: None,
            mvbox_handle: None,
            sentbox_handle: None,
            smtp_handle: None,
            events: receiver,
            running: Arc::new(AtomicBool::new(true)),
        };

        let ctx = account.context.clone();
        let running = account.running.clone();
        let imap_handle = std::thread::spawn(move || loop {
            use deltachat::job::*;

            while_running!(running, {
                perform_inbox_jobs(&ctx);
                perform_inbox_fetch(&ctx);
                while_running!(running, {
                    perform_inbox_idle(&ctx);
                });
            });
        });

        let ctx = account.context.clone();
        let running = account.running.clone();
        let sentbox_handle = std::thread::spawn(move || loop {
            use deltachat::job::*;

            while_running!(running, {
                perform_sentbox_fetch(&ctx);
                while_running!(running, {
                    perform_sentbox_idle(&ctx);
                });
            });
        });

        let ctx = account.context.clone();
        let running = account.running.clone();
        let mvbox_handle = std::thread::spawn(move || loop {
            use deltachat::job::*;

            while_running!(running, {
                perform_mvbox_fetch(&ctx);
                while_running!(running, {
                    perform_mvbox_idle(&ctx);
                });
            });
        });

        let ctx = account.context.clone();
        let running = account.running.clone();
        let smtp_handle = std::thread::spawn(move || loop {
            use deltachat::job::*;

            while_running!(running, {
                perform_smtp_jobs(&ctx);
                while_running!(running, {
                    perform_smtp_idle(&ctx);
                });
            });
        });

        account.imap_handle = Some(imap_handle);
        account.mvbox_handle = Some(mvbox_handle);
        account.sentbox_handle = Some(sentbox_handle);
        account.smtp_handle = Some(smtp_handle);

        Ok(account)
    }

    pub async fn logged_in(&self) -> bool {
        self.state.read().await.logged_in == Login::Success
    }

    pub async fn import(&self, path: &str) -> Result<()> {
        use deltachat::imex;

        imex::imex(&self.context, imex::ImexMode::ImportBackup, Some(path));

        let mut events = self.events.clone();
        while let Some(event) = events.next().await {
            match event {
                Event::ImexProgress(0) => {
                    bail!("Failed to import");
                }
                Event::ImexProgress(1000) => {
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
            .map_err(|err| anyhow!("{:?}", err))?;
        self.context
            .set_config(Config::MailPw, Some(password))
            .map_err(|err| anyhow!("{:?}", err))?;

        self.configure().await?;
        Ok(())
    }

    pub async fn configure(&self) -> Result<()> {
        info!("configure");

        let ctx = self.context.clone();
        task::spawn_blocking(move || {
            deltachat::configure::configure(&ctx);
        })
        .await;

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
        ls.chat_msg_ids = chat::get_chat_msgs(&self.context, chat_id, 0, None);
        ls.chat_msgs = Default::default();

        if let Some(chat_state) = chat_state {
            ls.selected_chat = Some(chat_state);
        }

        ls.chats.insert(chat_id, chat);

        Ok(())
    }

    pub async fn load_message_list(&self, start_index: usize, stop_index: usize) -> Result<()> {
        ensure!(start_index <= stop_index, "invalid indicies");
        {
            let mut ls = self.state.write().await;
            ls.chat_msgs_range = (start_index, stop_index);
        }

        refresh_message_list(self.context.clone(), self.state.clone(), None).await?;

        Ok(())
    }

    pub async fn send_text_message(&self, text: String) -> Result<()> {
        if let Some(chat_id) = self.state.read().await.selected_chat_id {
            chat::send_text_msg(&self.context, chat_id, text)
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
            let mut msg = message::Message::new(typ);
            msg.set_text(text);
            msg.set_file(path, mime.as_deref());

            chat::send_msg(&self.context, chat_id, &mut msg)
                .map_err(|err| anyhow!("failed to send message: {}", err))?;
        } else {
            bail!("no chat selected, can not send message");
        }

        Ok(())
    }

    pub fn subscribe<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
        local_state: Arc<RwLock<LocalState>>,
    ) where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
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
#[serde(rename_all = "snake_case")]
pub struct RemoteEvent {
    #[serde(rename = "type")]
    typ: String,
    event: String,
}

pub async fn refresh_chat_state(
    context: Arc<Context>,
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
        if let Some(index) = state.chatlist.get_index_for_id(chat_id) {
            state.chat_states.insert(index, chat_state);
        }
    }
    state.chats.insert(chat_id, chat);

    Ok(())
}

async fn load_chat_state(
    context: Arc<Context>,
    state: Arc<RwLock<AccountState>>,
    chat_id: ChatId,
) -> Result<(Chat, Option<ChatState>)> {
    let state = state.read().await;
    let chats = &state.chatlist;
    let chat = Chat::load_from_db(&context, chat_id)
        .map_err(|err| anyhow!("failed to load chats: {:?}", err))?;

    let chat_state = if let Some(index) = chats.get_index_for_id(chat_id) {
        let lot = chats.get_summary(&context, index, Some(&chat));

        let header = lot.get_text1().map(|s| s.to_string()).unwrap_or_default();
        let preview = lot.get_text2().map(|s| s.to_string()).unwrap_or_default();

        Some(ChatState {
            id: chat_id,
            name: chat.get_name().to_string(),
            subtitle: chat.get_subtitle(&context).to_string(),
            header,
            preview,
            timestamp: lot.get_timestamp(),
            state: lot.get_state().to_string(),
            profile_image: chat.get_profile_image(&context),
            can_send: chat.can_send(),
            chat_type: chat.get_type(),
            color: chat.get_color(&context),
            is_device_talk: chat.is_device_talk(),
            is_self_talk: chat.is_self_talk(),
            fresh_msg_cnt: chat_id.get_fresh_msg_cnt(&context),
        })
    } else {
        None
    };

    Ok((chat, chat_state))
}

pub async fn refresh_chat_list(
    context: Arc<Context>,
    state: Arc<RwLock<AccountState>>,
) -> Result<()> {
    let chatlist = task::spawn_blocking(move || {
        Chatlist::try_load(&context, 0, None, None)
            .map_err(|err| anyhow!("failed to load chats: {:?}", err))
    })
    .await?;

    std::mem::replace(&mut state.write().await.chatlist, chatlist);

    Ok(())
}

pub async fn refresh_message_list(
    context: Arc<Context>,
    state: Arc<RwLock<AccountState>>,
    chat_id: Option<ChatId>,
) -> Result<()> {
    let mut ls = state.write().await;
    let current_chat_id = ls.selected_chat_id.clone();
    if chat_id.is_some() && current_chat_id != chat_id {
        return Ok(());
    }

    let (start_index, stop_index) = ls.chat_msgs_range;
    info!(
        "loading chat messages {:?} - {}..{}",
        chat_id, start_index, stop_index
    );

    ls.chat_msg_ids = chat::get_chat_msgs(&context, current_chat_id.unwrap(), 0, None);

    // TODO: wrap in spawn_blocking
    let msgs = ls
        .chat_msg_ids
        .iter()
        .enumerate()
        .skip(start_index)
        .take(stop_index - start_index + 1)
        .map(|(i, msg_id)| {
            let msg = message::Message::load_from_db(&context, *msg_id)
                .map_err(|err| anyhow!("failed to load msg: {}: {}", msg_id, err))?;

            let from = Contact::load_from_db(&context, msg.get_from_id())
                .map_err(|err| anyhow!("failed to load contact: {}: {}", msg.get_from_id(), err))?;

            let chat_msg = ChatMessage {
                id: msg.get_id(),
                from_id: msg.get_from_id(),
                viewtype: msg.get_viewtype(),
                from_first_name: from.get_first_name().to_string(),
                from_profile_image: from.get_profile_image(&context),
                from_color: from.get_color(),
                starred: msg.is_starred(),
                state: msg.get_state(),
                text: msg.get_text(),
                timestamp: msg.get_sort_timestamp(),
                is_info: msg.is_info(),
                file: msg.get_file(&context),
                file_width: msg.get_width(),
                file_height: msg.get_height(),
            };
            Ok((i, chat_msg))
        })
        .collect::<Result<_>>()?;
    std::mem::replace(&mut ls.chat_msgs, msgs);

    Ok(())
}
