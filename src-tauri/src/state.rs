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
                            match account.configure().await {
                                Ok(()) => {
                                    info!("configured");
                                    account.context.start_io().await;
                                    accounts.insert(account_name.to_string(), account);
                                }
                                Err(err) => info!("Account could not be configured: {}", err),
                            }
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

        // Select the first one by default
        let selected_account = accounts.keys().next().cloned();
        info!("selecting account {:?}", selected_account);
        if let Some(ref selected) = selected_account {
            accounts.get(selected).unwrap();
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
        self.send(writer, response).await?;
        Ok(())
    }

    pub async fn send_event<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
        account: u32,
        event: shared::Event,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        self.send(writer, Response::Event { account, event })
            .await?;
        Ok(())
    }

    pub async fn send<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
        response: Response,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
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
        let (selected_chat_id, selected_chat) =
            if let Some(ref account_name) = self.selected_account {
                let account = self
                    .accounts
                    .get(account_name)
                    .expect("invalid account state");

                let state = account.state.read().await;
                (state.selected_chat_id.clone(), state.selected_chat.clone())
            } else {
                (None, None)
            };

        Response::RemoteUpdate {
            state: State {
                shared: SharedState {
                    accounts,
                    errors,
                    selected_account: self.selected_account.clone(),
                    selected_chat_id: selected_chat_id.map(|s| s.to_u32()),
                    selected_chat,
                },
            },
        }
    }
}
