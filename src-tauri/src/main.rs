#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::{ensure, Result};
use async_std::net::{TcpListener, TcpStream};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::Error;
use deltachat::{chat::ChatId, message::MsgId};
use futures::StreamExt;
use log::{info, warn};
use num_traits::FromPrimitive;
use shared::*;

use dc40_backend::{account::*, state::*};

fn main() {
    femme::start(log::LevelFilter::Info).unwrap();

    std::thread::spawn(|| {
        let addr = "127.0.0.1:8080";

        task::block_on(async move {
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Failed to bind");
            info!("Listening on: {}", addr);

            match LocalState::new().await {
                Ok(created_state) => {
                    info!("configured");
                    info!("Restored local state");
                    let local_state = Arc::new(RwLock::new(created_state));
                    while let Ok((stream, _)) = listener.accept().await {
                        let local_state = local_state.clone();
                        task::spawn(async move {
                            if let Err(err) = accept_connection(stream, local_state).await {
                                if let Some(err) = err.downcast_ref::<Error>() {
                                    match err {
                                        Error::ConnectionClosed
                                        | Error::Protocol(_)
                                        | Error::Utf8 => {}
                                        err => warn!("Error processing connection: {:?}", err),
                                    }
                                } else {
                                    warn!("Error processing connection: {:?}", err);
                                }
                            }
                        });
                    }
                }
                Err(err) => info!("Local state could not be restored: {}", err),
            }
        });
    });

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
        if msg.is_close() {
            info!("closing connection");
            return Ok(());
        }

        if !msg.is_binary() {
            warn!("ignoring unknown message {:?}", &msg);
            continue;
        }
        let parsed: std::result::Result<Request, _> = bincode::deserialize(&msg.into_data());
        info!("request: {:?}", &parsed);
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
                        let ls = local_state.write().await;

                        // send base state
                        ls.send_update(write.clone()).await?;

                        if let Some(account) = ls.accounts.get(&email) {
                            // chat list
                            let (range, len, chats) = account.load_chat_list(0, 10).await?;
                            ls.send(write.clone(), Response::ChatList { range, len, chats })
                                .await?;

                            // send selected chat if exists
                            if let Some(_selected_chat) =
                                account.state.read().await.selected_chat_id
                            {
                                let (chat_id, range, items, messages) =
                                    account.load_message_list(None).await?;

                                ls.send(
                                    write.clone(),
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
                    }
                }
                Request::Import { path, email } => {
                    ensure!(!email.is_empty(), "Missing email");

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
                        let ls = local_state.write().await;
                        if let Some(account) = ls.accounts.get(&email) {
                            let (range, len, chats) = account.load_chat_list(0, 10).await?;
                            ls.send(write.clone(), Response::ChatList { range, len, chats })
                                .await?;
                        }
                    }
                    let local_state = local_state.read().await;
                    local_state.send_update(write.clone()).await?;
                }
                Request::SelectChat { account, chat_id } => {
                    let ls = local_state.write().await;
                    if let Some(account) = ls.accounts.get(&account) {
                        let chat = ChatId::new(chat_id);
                        account.select_chat(chat).await?;
                        ls.send_update(write.clone()).await?;

                        let (chat_id, range, items, messages) =
                            account.load_message_list(None).await?;

                        ls.send(
                            write.clone(),
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
                        info!("Loading chat list for account: {:?}", ls.selected_account);
                        match account.load_chat_list(start_index, stop_index).await {
                            Ok((range, len, chats)) => {
                                ls.send(write.clone(), Response::ChatList { range, len, chats })
                                    .await?;
                            }
                            Err(err) => {
                                info!("Could not load chat list: {}", err);
                                // send an empty chat list to be handled by frontend
                                let chats = Vec::with_capacity(0);
                                ls.send(
                                    write.clone(),
                                    Response::ChatList {
                                        range: (start_index, stop_index),
                                        len: 0,
                                        chats: chats,
                                    },
                                )
                                .await?;
                            }
                        }
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
                        let range = if start_index == 0 && stop_index == 0 {
                            None
                        } else {
                            Some((start_index, stop_index))
                        };
                        let (chat_id, range, items, messages) =
                            account.load_message_list(range).await?;

                        ls.send(
                            write.clone(),
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
                Request::SelectAccount { account } => {
                    info!("selecting account {}", account);
                    let mut ls = local_state.write().await;
                    ls.selected_account = Some(account.clone());
                    ls.send(
                        write.clone(),
                        Response::Account {
                            account: account.to_string(),
                        },
                    )
                    .await?;
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
                        account
                            .send_file_message(
                                Viewtype::from_i32(typ as i32).unwrap(),
                                path,
                                text,
                                mime,
                            )
                            .await?;
                        ls.send_update(write.clone()).await?;
                    }
                }
                Request::CreateChatById { id } => {
                    info!("creating chat by id {:?}", id);

                    let ls = local_state.read().await;
                    if let Some(account) = ls
                        .selected_account
                        .as_ref()
                        .and_then(|a| ls.accounts.get(a))
                    {
                        account.create_chat_by_id(MsgId::new(id)).await?;
                        ls.send_update(write.clone()).await?;
                    }
                }
                Request::MaybeNetwork => {
                    info!("maybe network");

                    let ls = local_state.read().await;
                    if let Some(account) = ls
                        .selected_account
                        .as_ref()
                        .and_then(|a| ls.accounts.get(a))
                    {
                        account.maybe_network().await;
                    }
                }
            },
            Err(err) => warn!("invalid msg {}", err),
        }
    }

    Ok(())
}
