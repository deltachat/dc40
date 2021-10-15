use std::collections::HashMap;

use anyhow::{anyhow, Result};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_std::{path::Path, prelude::*};
use async_tungstenite::tungstenite::{Error, Message};
use broadcaster::BroadcastChannel;
use deltachat::chat::{Chat, ChatId};
use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::{message, EventType};
use futures::future::join_all;
use futures::sink::SinkExt;
use futures::stream::{self, StreamExt};
use itertools::Itertools;
use log::*;
use num_traits::FromPrimitive;
use shared::*;

use crate::account::*;

#[derive(Debug, Clone)]
pub struct LocalState {
    inner: Arc<RwLock<LocalStateInner>>,
    events: BroadcastChannel<deltachat::Event>,
}

#[derive(Debug)]
struct LocalStateInner {
    account_states: HashMap<u32, Account>,
    accounts: deltachat::accounts::Accounts,
    errors: Vec<anyhow::Error>,
}

sa::assert_impl_all!(LocalState: Send);

impl LocalState {
    pub async fn new() -> Result<Self> {
        let inner = LocalStateInner::new().await?;

        let receiver = BroadcastChannel::new();
        let sender = receiver.clone();
        let mut events = inner.accounts.get_event_emitter().await;

        task::spawn(async move {
            while let Ok(Some(event)) = events.recv().await {
                if let Err(err) = sender.send(&event).await {
                    error!("Failed to send event: {:?}", err);
                }
            }
        });

        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
            events: receiver,
        })
    }

    async fn with_account_state<F>(&self, id: u32, f: F)
    where
        F: FnOnce(&mut crate::account::AccountState),
    {
        let ls = self.inner.read().await;
        let account = ls.account_states.get(&id).expect("missing account");

        let state = &mut account.state.write().await;
        f(state);
    }

    pub async fn subscribe_all<T>(&self, writer: Arc<RwLock<T>>) -> Result<()>
    where
        T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static,
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let mut events = self.events.clone();
        let ls = self.clone();

        task::spawn(async move {
            while let Some(event) = events.next().await {
                let ctx = ls
                    .inner
                    .read()
                    .await
                    .accounts
                    .get_account(event.id)
                    .await
                    .unwrap();

                let res = match event.typ {
                    EventType::ConfigureProgress { progress, .. } => {
                        if progress == 0 {
                            ls.with_account_state(event.id, |state| {
                                state.logged_in = Login::Error("failed to login".into());
                            })
                            .await;
                            ls.send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::Configure(shared::Progress::Error),
                            )
                            .await
                        } else {
                            let p = if progress == 1000 {
                                shared::Progress::Success
                            } else {
                                ls.with_account_state(event.id, |state| {
                                    state.logged_in = Login::Progress(progress);
                                })
                                .await;
                                shared::Progress::Step(progress)
                            };
                            ls.send_event(writer.clone(), event.id, shared::Event::Configure(p))
                                .await
                        }
                    }
                    EventType::ImexProgress(progress) => {
                        if progress == 0 {
                            ls.with_account_state(event.id, |state| {
                                state.logged_in = Login::Error("failed to import".into());
                            })
                            .await;
                            ls.send_event(
                                writer.clone(),
                                event.id,
                                shared::Event::Imex(shared::Progress::Error),
                            )
                            .await
                        } else {
                            let p = if progress == 1000 {
                                shared::Progress::Success
                            } else {
                                ls.with_account_state(event.id, |state| {
                                    state.logged_in = Login::Progress(progress);
                                })
                                .await;
                                shared::Progress::Step(progress)
                            };
                            ls.send_event(writer.clone(), event.id, shared::Event::Imex(p))
                                .await
                        }
                    }
                    EventType::ImapConnected(_) | EventType::SmtpConnected(_) => {
                        info!("logged in");
                        ls.with_account_state(event.id, |state| {
                            state.logged_in = Login::Success;
                        })
                        .await;
                        ls.send_event(writer.clone(), event.id, shared::Event::Connected)
                            .await
                    }
                    EventType::IncomingMsg { chat_id, msg_id } => {
                        let load = || async {
                            let msg = message::Message::load_from_db(&ctx, msg_id).await.map_err(
                                |err| anyhow!("failed to load msg: {}: {}", msg_id, err),
                            )?;
                            let chat = Chat::load_from_db(&ctx, chat_id)
                                .await
                                .map_err(|err| anyhow!("failed to load chat: {:?}", err))?;

                            ls.send_event(
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
                        ls.send_event(
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
                        ls.send_event(
                            writer.clone(),
                            event.id,
                            shared::Event::Log(shared::Log::Info(msg)),
                        )
                        .await
                    }
                    EventType::Warning(msg) => {
                        warn!("{}", msg);
                        ls.send_event(
                            writer.clone(),
                            event.id,
                            shared::Event::Log(shared::Log::Warning(msg)),
                        )
                        .await
                    }
                    EventType::Error(msg) => {
                        error!("{}", msg);
                        ls.send_event(
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

        Ok(())
    }

    pub async fn add_account(&self) -> Result<(u32, Context)> {
        let mut ls = self.inner.write().await;
        let id = ls.accounts.add_account().await?;
        let ctx = ls.accounts.get_account(id).await.unwrap();
        let account = Account::new()?;

        ls.account_states.insert(id, account);

        Ok((id, ctx.clone()))
    }

    pub async fn login(&self, id: u32, ctx: &Context, email: &str, password: &str) -> Result<()> {
        let res = self
            .inner
            .read()
            .await
            .account_states
            .get(&id)
            .unwrap()
            .login(&ctx, &email, &password)
            .await;
        if let Err(err) = res {
            let mut ls = self.inner.write().await;
            ls.errors.push(err);
            ls.account_states.remove(&id);
            ls.accounts.remove_account(id).await?;
        }

        Ok(())
    }

    pub async fn send_account_details<T>(&self, id: u32, writer: Arc<RwLock<T>>) -> Result<()>
    where
        T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static,
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let ls = self.inner.write().await;
        let ctx = ls.accounts.get_account(id).await.unwrap();

        ls.send_update(writer.clone()).await?;

        if let Some(account) = ls.account_states.get(&id) {
            // chat list
            let (range, len, chats) = account.load_chat_list(&ctx, 0, 10).await?;
            send(writer.clone(), Response::ChatList { range, len, chats }).await?;

            // send selected chat if exists
            if let Some(_selected_chat) = account.state.read().await.selected_chat_id {
                let (chat_id, range, items, messages) =
                    account.load_message_list(&ctx, None).await?;

                send(
                    writer,
                    Response::MessageList {
                        chat_id,
                        range,
                        items,
                        messages,
                    },
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn import(&self, ctx: &Context, id: u32, path: &Path) -> Result<()> {
        let res = self
            .inner
            .read()
            .await
            .account_states
            .get(&id)
            .unwrap()
            .import(&ctx, path)
            .await;
        if let Err(err) = res {
            let mut ls = self.inner.write().await;
            ls.errors.push(err);
            ls.account_states.remove(&id);
            ls.accounts.remove_account(id).await?;
        }

        Ok(())
    }

    pub async fn send_update<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        self.inner.read().await.send_update(writer).await
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
        self.inner
            .read()
            .await
            .send_event(writer, account, event)
            .await
    }

    pub async fn select_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.select_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn pin_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.pin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn unpin_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.unpin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn archive_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.archive_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn unarchive_chat(&self, account_id: u32, chat_id: u32) -> Result<Response> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.unpin_chat(&ctx, chat).await?;

            let (chat_id, range, items, messages) = account.load_message_list(&ctx, None).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn accept_contact_request(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.accept_contact_request(&ctx, chat).await?;

            Ok(())
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn block_contact(&self, account_id: u32, chat_id: u32) -> Result<()> {
        let ls = self.inner.write().await;
        if let Some(account) = ls.account_states.get(&account_id) {
            let ctx = ls.accounts.get_account(account_id).await.unwrap();
            let chat = ChatId::new(chat_id);
            account.block_contact(&ctx, chat).await?;

            Ok(())
        } else {
            Err(anyhow!("invalid account: {}-{}", account_id, chat_id))
        }
    }

    pub async fn load_chat_list(&self, start_index: usize, stop_index: usize) -> Result<Response> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            info!("Loading chat list");
            match account.load_chat_list(&ctx, start_index, stop_index).await {
                Ok((range, len, chats)) => Ok(Response::ChatList { range, len, chats }),
                Err(err) => {
                    info!("Could not load chat list: {}", err);
                    // send an empty chat list to be handled by frontend
                    Ok(Response::ChatList {
                        range: (start_index, stop_index),
                        len: 0,
                        chats: Vec::new(),
                    })
                }
            }
        } else {
            Err(anyhow!("no selected account"))
        }
    }

    pub async fn send_contacts<T>(&self, writer: Arc<RwLock<T>>) -> Result<()>
    where
        T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static,
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let ls = self.inner.read().await;
        if let Some((_, ctx)) = ls.get_selected_account().await {
            let query: Option<&'static str> = None;
            let contact_ids = deltachat::contact::Contact::get_all(&ctx, 0, query).await?;
            info!("Contact-list: {:?}", contact_ids);
            let contacts = stream::iter(contact_ids)
                .then(|id| Contact::load_from_db(&ctx, id))
                .filter_map(|contact| async {
                    match contact {
                        Ok(contact) => Some(ContactInfo {
                            id: contact.id,
                            mail: contact.get_addr().to_owned(),
                            display_name: contact.get_display_name().to_owned(),
                        }),
                        Err(_) => None,
                    }
                })
                .collect::<Vec<ContactInfo>>()
                .await;

            send(writer, Response::Contacts(contacts)).await?;
            Ok(())
        } else {
            Err(anyhow!("no selected account"))
        }
    }

    pub async fn load_message_list(&self, range: Option<(usize, usize)>) -> Result<Response> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            let (chat_id, range, items, messages) = account.load_message_list(&ctx, range).await?;

            Ok(Response::MessageList {
                chat_id,
                range,
                items,
                messages,
            })
        } else {
            Err(anyhow!("no selected account"))
        }
    }

    pub async fn select_account(&self, account_id: u32) -> Result<Response> {
        let mut ls = self.inner.write().await;
        ls.select_account(account_id).await?;
        if let Some((account, _ctx)) = ls.get_selected_account().await {
            let state = account.state.read().await;

            Ok(Response::Account {
                account: account_id,
                chat_id: state.selected_chat_id.map(|id| id.to_u32()),
                chat: state.selected_chat.as_ref().cloned(),
            })
        } else {
            Err(anyhow!("failed to select account"))
        }
    }

    pub async fn send_text_message(&self, text: String) -> Result<()> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            account.send_text_message(&ctx, text).await?;
            Ok(())
        } else {
            Err(anyhow!("no account selected"))
        }
    }

    pub async fn send_file_message(
        &self,
        typ: Viewtype,
        path: String,
        text: Option<String>,
        mime: Option<String>,
    ) -> Result<()> {
        let ls = self.inner.read().await;
        if let Some((account, ctx)) = ls.get_selected_account().await {
            account
                .send_file_message(
                    &ctx,
                    Viewtype::from_i32(typ as i32).unwrap(),
                    path,
                    text,
                    mime,
                )
                .await?;
            Ok(())
        } else {
            Err(anyhow!("no account selected"))
        }
    }

    pub async fn maybe_network(&self) -> Result<()> {
        let ls = self.inner.read().await;
        ls.accounts.maybe_network().await;
        Ok(())
    }
}

impl LocalStateInner {
    pub async fn new() -> Result<Self> {
        info!("restoring local state");

        // load accounts from default dir
        let mut account_states = HashMap::new();
        let accounts =
            deltachat::accounts::Accounts::new("cool_os".to_string(), HOME_DIR.clone()).await?;
        for id in &accounts.get_all().await {
            let state = Account::new()?;
            account_states.insert(*id, state);
        }

        info!("loaded state");

        accounts.start_io().await;

        info!("started io");

        Ok(Self {
            accounts,
            account_states,
            errors: Vec::new(),
        })
    }

    pub async fn get_selected_account(&self) -> Option<(&Account, deltachat::context::Context)> {
        if let Some(ctx) = self.accounts.get_selected_account().await {
            let id = ctx.get_id();
            if let Some(account) = self.account_states.get(&id) {
                Some((account, ctx))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub async fn select_account(&mut self, id: u32) -> Result<()> {
        self.accounts.select_account(id).await?;
        Ok(())
    }

    pub async fn send_update<T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static>(
        &self,
        writer: Arc<RwLock<T>>,
    ) -> Result<()>
    where
        T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
    {
        let response = self.to_response().await;
        send(writer, response).await?;
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
        send(writer, Response::Event { account, event }).await?;
        Ok(())
    }

    pub async fn get_selected_account_id(&self) -> Option<u32> {
        if let Some(ctx) = self.accounts.get_selected_account().await {
            Some(ctx.get_id())
        } else {
            None
        }
    }

    pub async fn get_selected_account_state(&self) -> Option<&Account> {
        if let Some(id) = self.get_selected_account_id().await {
            self.account_states.get(&id)
        } else {
            None
        }
    }

    pub async fn to_response(&self) -> Response {
        let mut accounts = HashMap::with_capacity(self.account_states.len());
        for (id, account) in self.account_states.iter() {
            let account = &account.state.read().await;
            let ctx = self.accounts.get_account(*id).await.unwrap();

            use deltachat::config::Config;
            let email = ctx.get_config(Config::Addr).await.unwrap().unwrap();
            let profile_image = ctx
                .get_config(Config::Selfavatar)
                .await
                .unwrap()
                .map(Into::into);
            let display_name = ctx.get_config(Config::Displayname).await.unwrap();

            accounts.insert(
                *id,
                SharedAccountState {
                    logged_in: account.logged_in.clone(),
                    email,
                    profile_image,
                    display_name,
                },
            );
        }

        let errors = self.errors.iter().map(|e| e.to_string()).collect();
        let (selected_chat_id, selected_chat) =
            if let Some(account) = self.get_selected_account_state().await {
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
                    selected_account: self.get_selected_account_id().await,
                    selected_chat_id: selected_chat_id.map(|s| s.to_u32()),
                    selected_chat,
                },
            },
        }
    }
}

pub async fn send<T>(writer: Arc<RwLock<T>>, response: Response) -> Result<()>
where
    T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static,
    T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
{
    writer
        .write()
        .await
        .send(Message::binary(bincode::serialize(&response).unwrap()))
        .await
        .map_err(Into::into)
}
