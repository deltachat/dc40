use std::collections::HashMap;

use anyhow::Result;
use async_std::{
    sync::{Arc, RwLock},
    task,
};
use async_tungstenite::tungstenite::Message;
use futures::sink::SinkExt;
use log::{info, warn};
use shared::*;

use crate::account::*;

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

impl LocalState {
    pub async fn send_update<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let response = self.to_response().await;

        writer
            .write()
            .await
            .send(Message::binary(bincode::serialize(&response).unwrap()))
            .await
            .map_err(Into::into)
    }

    pub async fn to_response(&self) -> Response {
        let mut accounts = HashMap::with_capacity(self.accounts.len());
        for (email, account) in self.accounts.iter() {
            let account = &account.state.read().await;
            accounts.insert(
                email.clone(),
                SharedAccountState {
                    logged_in: account.logged_in.clone(),
                    email: account.email.clone(),
                },
            );
        }

        let errors = self.errors.iter().map(|e| e.to_string()).collect();
        let (
            chats,
            selected_chat_length,
            selected_chat_id,
            selected_chat,
            selected_messages_length,
            selected_messages_range,
            messages,
        ) = if let Some(ref account_name) = self.selected_account {
            let account = self
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
                state.chat_msgs_range,
                state.chat_msgs.clone(),
            )
        } else {
            (Default::default(), 0, None, None, 0, (0, 0), Vec::new())
        };

        Response::RemoteUpdate {
            state: State {
                shared: SharedState {
                    accounts,
                    errors,
                    selected_account: self.selected_account.clone(),
                    selected_chat_id: selected_chat_id.map(|s| s.to_u32()),
                    selected_chat,
                    selected_chat_length,
                    chats,
                    messages,
                    selected_messages_length,
                    selected_messages_range,
                },
            },
        }
    }
}
