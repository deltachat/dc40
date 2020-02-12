use anyhow::Result;
use async_std::net::{TcpListener, TcpStream};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::Error;
use futures::StreamExt;
use log::{info, warn};

mod account;
mod state;

use account::*;
use state::*;

fn main() {
    femme::start(log::LevelFilter::Info).unwrap();

    let addr = "127.0.0.1:8080";

    task::block_on(async move {
        let local_state = Arc::new(RwLock::new(LocalState::new().await.unwrap()));

        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&addr).await;
        let listener = try_socket.expect("Failed to bind");
        info!("Listening on: {}", addr);

        while let Ok((stream, _)) = listener.accept().await {
            let local_state = local_state.clone();
            task::spawn(async move {
                if let Err(err) = accept_connection(stream, local_state).await {
                    match err.downcast_ref::<Error>() {
                        Some(Error::ConnectionClosed)
                        | Some(Error::Protocol(_))
                        | Some(Error::Utf8) => (),
                        err => warn!("Error processing connection: {:?}", err),
                    }
                }
            });
        }
    });
}

async fn accept_connection(stream: TcpStream, local_state: Arc<RwLock<LocalState>>) -> Result<()> {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, mut read) = ws_stream.split();
    let write = Arc::new(RwLock::new(write));

    {
        let ls = local_state.read().await;
        // send initial state
        ls.send_update(write.clone()).await?;

        // setup account subscriptions
        for account in ls.accounts.values() {
            account.subscribe(write.clone(), local_state.clone());
        }
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;
        info!("got msg: {:?}", &msg);
        if msg.is_text() {
            let parsed: std::result::Result<Request, _> = serde_json::from_str(msg.to_text()?);
            match parsed {
                Ok(request) => match request {
                    Request::Login {
                        email, password, ..
                    } => {
                        let email = email.to_lowercase();
                        {
                            let account = Account::new(&email).await?;
                            account.subscribe(write.clone(), local_state.clone());
                            let mut local_state = local_state.write().await;
                            local_state.accounts.insert(email.clone(), account);
                        };

                        {
                            let res = local_state
                                .read()
                                .await
                                .accounts
                                .get(&email)
                                .unwrap()
                                .login(&email, &password)
                                .await;
                            if let Err(err) = res {
                                let mut ls = local_state.write().await;
                                ls.errors.push(err);
                                ls.accounts.remove(&email);
                            }
                        }

                        {
                            if let Some(account) = local_state.read().await.accounts.get(&email) {
                                account.load_chat_list(0, 10).await?;
                            }
                        }
                        let local_state = local_state.read().await;
                        local_state.send_update(write.clone()).await?;
                    }
                    Request::Import { path, email } => {
                        {
                            let account = Account::new(&email).await?;
                            account.subscribe(write.clone(), local_state.clone());
                            let mut local_state = local_state.write().await;
                            local_state.accounts.insert(email.clone(), account);
                        };

                        {
                            let res = local_state
                                .read()
                                .await
                                .accounts
                                .get(&email)
                                .unwrap()
                                .import(&path)
                                .await;
                            if let Err(err) = res {
                                let mut ls = local_state.write().await;
                                ls.errors.push(err);
                                ls.accounts.remove(&email);
                            }
                        }

                        {
                            if let Some(account) = local_state.read().await.accounts.get(&email) {
                                account.load_chat_list(0, 10).await?;
                            }
                        }
                        let local_state = local_state.read().await;
                        local_state.send_update(write.clone()).await?;
                    }
                    Request::SelectChat { account, chat_id } => {
                        let ls = local_state.write().await;
                        if let Some(account) = ls.accounts.get(&account) {
                            account.select_chat(chat_id).await?;
                            ls.send_update(write.clone()).await?;

                            account.load_message_list(0, 20).await?;
                            ls.send_update(write.clone()).await?;
                        }
                    }
                    Request::LoadChatList {
                        start_index,
                        stop_index,
                    } => {
                        let ls = local_state.read().await;
                        if let Some(account) = ls
                            .selected_account
                            .as_ref()
                            .and_then(|a| ls.accounts.get(a))
                        {
                            account.load_chat_list(start_index, stop_index).await?;
                            ls.send_update(write.clone()).await?;
                        }
                    }
                    Request::LoadMessageList {
                        start_index,
                        stop_index,
                    } => {
                        let ls = local_state.read().await;
                        if let Some(account) = ls
                            .selected_account
                            .as_ref()
                            .and_then(|a| ls.accounts.get(a))
                        {
                            account.load_message_list(start_index, stop_index).await?;
                            ls.send_update(write.clone()).await?;
                        }
                    }
                    Request::SelectAccount { account } => {
                        info!("selecting account {}", account);

                        let mut ls = local_state.write().await;
                        ls.selected_account = Some(account);
                        ls.send_update(write.clone()).await?;
                    }
                    Request::SendTextMessage { text } => {
                        let ls = local_state.read().await;
                        if let Some(account) = ls
                            .selected_account
                            .as_ref()
                            .and_then(|a| ls.accounts.get(a))
                        {
                            account.send_text_message(text).await?;
                            ls.send_update(write.clone()).await?;
                        }
                    }
                    Request::SendFileMessage {
                        typ,
                        path,
                        text,
                        mime,
                    } => {
                        let ls = local_state.read().await;
                        if let Some(account) = ls
                            .selected_account
                            .as_ref()
                            .and_then(|a| ls.accounts.get(a))
                        {
                            account.send_file_message(typ, path, text, mime).await?;
                            ls.send_update(write.clone()).await?;
                        }
                    }
                },
                Err(err) => warn!("invalid msg {}", err),
            }
        }
    }

    Ok(())
}
