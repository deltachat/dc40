use std::collections::HashMap;

use anyhow::Result;
use async_std::{
    sync::{Arc, RwLock},
    task,
};
use async_tungstenite::tungstenite::Message;
use deltachat::{chat::ChatId, constants::Viewtype, message::MsgId};
use futures::sink::SinkExt;
use log::{info, warn};
use serde::{Deserialize, Serialize};

use crate::account::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Request {
    #[serde(rename = "LOGIN")]
    Login {
        email: String,
        password: String,
        remote: bool,
    },
    #[serde(rename = "IMPORT")]
    Import { path: String, email: String },
    #[serde(rename = "SELECT_CHAT")]
    SelectChat { account: String, chat_id: ChatId },
    #[serde(rename = "LOAD_CHAT_LIST")]
    LoadChatList {
        start_index: usize,
        stop_index: usize,
    },
    #[serde(rename = "LOAD_MESSAGE_LIST")]
    LoadMessageList {
        start_index: usize,
        stop_index: usize,
    },
    #[serde(rename = "SELECT_ACCOUNT")]
    SelectAccount { account: String },
    #[serde(rename = "SEND_TEXT_MESSAGE")]
    SendTextMessage { text: String },
    #[serde(rename = "SEND_FILE_MESSAGE")]
    SendFileMessage {
        typ: Viewtype,
        path: String,
        text: Option<String>,
        mime: Option<String>,
    },
    #[serde(rename = "CREATE_CHAT_BY_ID")]
    CreateChatById { id: MsgId },
    #[serde(rename = "MAYBE_NETWORK")]
    MaybeNetwork,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Response {
    #[serde(rename = "REMOTE_UPDATE")]
    RemoteUpdate { state: State },
}

#[derive(Debug, Default)]
pub struct LocalState {
    pub accounts: HashMap<String, Account>,
    pub errors: Vec<anyhow::Error>,
    pub selected_account: Option<String>,
}

impl LocalState {
    pub async fn new() -> Result<Self> {
        info!("restoring local state");

        // load accounts from default dir
        let mut accounts = HashMap::new();

        let matcher = format!("{}/*.sqlite", HOME_DIR.display());
        for entry in task::spawn_blocking(move || glob::glob(&matcher)).await? {
            match entry {
                Ok(path) => {
                    match path.file_stem() {
                        Some(account_name) => {
                            let account_name = match account_name.to_str() {
                                Some(name) => name,
                                None => {
                                    warn!("Ignoring invalid filename: '{}'", path.display());
                                    continue;
                                }
                            };

                            // Load account
                            info!(
                                "Loading account: '{}' from '{}'",
                                account_name,
                                path.display()
                            );

                            let account = Account::new(account_name).await?;
                            // attempt to configure it
                            account.configure().await?;
                            info!("configured");
                            account.context.start_io().await;

                            accounts.insert(account_name.to_string(), account);
                        }
                        None => {
                            warn!("Ignoring invalid filename: '{}'", path.display());
                        }
                    }
                }
                Err(err) => {
                    warn!("Ignoring invalid file: {}", err);
                }
            }
        }

        info!("selecting account");

        // Select the first one by default
        let selected_account = accounts.keys().next().cloned();
        if let Some(ref selected) = selected_account {
            accounts
                .get(selected)
                .unwrap()
                .load_chat_list(0, 10)
                .await?;
        }
        info!("loaded state");

        Ok(LocalState {
            accounts,
            errors: Vec::new(),
            selected_account,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub shared: SharedState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SharedState {
    pub accounts: HashMap<String, SharedAccountState>,
    pub errors: Vec<String>,
    pub selected_account: Option<String>,
    pub selected_chat_id: Option<ChatId>,
    pub selected_chat: Option<ChatState>,
    pub selected_chat_length: usize,
    pub chats: Vec<ChatState>,
    pub selected_messages_length: usize,
    pub messages: HashMap<usize, ChatMessage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SharedAccountState {
    pub logged_in: Login,
    pub email: String,
}

impl Response {
    pub async fn from_local_state(state: &LocalState) -> Self {
        let mut accounts = HashMap::with_capacity(state.accounts.len());
        for (email, account) in state.accounts.iter() {
            let account = &account.state.read().await;
            accounts.insert(
                email.clone(),
                SharedAccountState {
                    logged_in: account.logged_in.clone(),
                    email: account.email.clone(),
                },
            );
        }

        let errors = state.errors.iter().map(|e| e.to_string()).collect();
        let (
            chats,
            selected_chat_length,
            selected_chat_id,
            selected_chat,
            selected_messages_length,
            messages,
        ) = if let Some(ref account_name) = state.selected_account {
            let account = state
                .accounts
                .get(account_name)
                .expect("invalid account state");

            let state = account.state.read().await;

            let mut chat_states: Vec<_> = state
                .chat_states
                .iter()
                .map(|(_id, state)| state.clone())
                .collect();
            chat_states.sort_unstable_by_key(|state| state.index);

            (
                chat_states,
                state.chatlist.len(),
                state.selected_chat_id.clone(),
                state.selected_chat.clone(),
                state.chat_msg_ids.len(),
                state.chat_msgs.clone(),
            )
        } else {
            (Default::default(), 0, None, None, 0, HashMap::new())
        };

        Response::RemoteUpdate {
            state: State {
                shared: SharedState {
                    accounts,
                    errors,
                    selected_account: state.selected_account.clone(),
                    selected_chat_id,
                    selected_chat,
                    selected_chat_length,
                    chats,
                    messages,
                    selected_messages_length,
                },
            },
        }
    }
}

impl LocalState {
    pub async fn send_update<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let response = Response::from_local_state(self).await;

        writer
            .write()
            .await
            .send(Message::text(serde_json::to_string(&response).unwrap()))
            .await
            .map_err(Into::into)
    }
}
