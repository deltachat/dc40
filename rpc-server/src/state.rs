use anyhow::Result;
use async_std::{
    sync::{Arc, RwLock},
    task,
};
use async_tungstenite::tungstenite::Message;
use deltachat::chat::ChatId;
use futures::sink::SinkExt;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            dbg!(&entry);
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
            accounts.get(selected).unwrap().load_chats().await?;
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
    pub accounts: HashMap<String, AccountState>,
    pub errors: Vec<String>,
    pub selected_account: Option<String>,
}

impl Response {
    pub async fn from_local_state(state: &LocalState) -> Self {
        let mut accounts = HashMap::with_capacity(state.accounts.len());
        for (email, account) in state.accounts.iter() {
            accounts.insert(email.clone(), account.state.read().await.clone());
        }

        let errors = state.errors.iter().map(|e| e.to_string()).collect();

        Response::RemoteUpdate {
            state: State {
                shared: SharedState {
                    accounts,
                    errors,
                    selected_account: state.selected_account.clone(),
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
