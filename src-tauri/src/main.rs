#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use async_std::net::{TcpListener, TcpStream};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::{Error, Message};
use futures::StreamExt;
use log::{error, info, warn};
use num_traits::FromPrimitive;
use shared::*;

use dc40_backend::state::*;

fn main() {
    femme::with_level(log::LevelFilter::Info);

    std::thread::spawn(|| {
        let addr = "127.0.0.1:8080";

        task::block_on(async move {
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Failed to bind");
            info!("Listening on: {}", addr);

            match LocalState::new().await {
                Ok(local_state) => {
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

async fn accept_connection(stream: TcpStream, local_state: LocalState) -> Result<()> {
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

    info!("send update");
    local_state.send_update(write.clone()).await?;

    info!("subscribe_all");
    local_state.subscribe_all(write.clone()).await?;

    info!("start loop");

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

        match &parsed {
            Ok(req) => match req {
                Request::Import { .. } => info!("request: Import"),
                req => info!("request: {:?}", req),
            },
            Err(_) => error!("couldn't deserialize request"),
        }

        match parsed {
            Ok(request) => {
                if let Err(err) = process_request(request, write.clone(), &local_state).await {
                    warn!("error processing request: {:?}", err);
                }
            }
            Err(err) => warn!("invalid msg {}", err),
        }
    }

    Ok(())
}

async fn process_request<T>(
    request: Request,
    write: Arc<RwLock<T>>,
    local_state: &LocalState,
) -> Result<()>
where
    T: futures::sink::Sink<Message> + Unpin + Sync + Send + 'static,
    T::Error: std::fmt::Debug + std::error::Error + Send + Sync,
{
    match request {
        Request::Login { email, password } => {
            let email = email.to_lowercase();
            let (id, ctx) = local_state.add_account().await?;

            task::spawn(async {
                if let Ok(listener) = TcpListener::bind("127.0.0.1:6969").await {
                    let t = listener.accept().await;
                    if let Ok((stream, _)) = t {
                        let mut wasm_stream =
                            async_tungstenite::accept_async(stream).await.unwrap();
                        if let Some(Ok(Message::Binary(import))) = wasm_stream.next().await {
                            info!("received update");
                        };
                    }
                }
            });

            
            /*local_state.login(id, &ctx, &email, &password).await?;

            local_state
                .send_account_details(&ctx, id, write.clone())
                .await?; */
        }
        Request::Import {} => {
            let (id, ctx) = local_state.add_account().await?;

            //local_state.import(&ctx, id, &path).await?;

            local_state
                .send_account_details(&ctx, id, write.clone())
                .await?;
        }
        Request::SelectChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.select_chat(id, chat_id).await?;
            local_state.send_update(write.clone()).await?;
            send(write.clone(), resp).await?;
        }
        Request::LoadChatList {
            start_index,
            stop_index,
        } => {
            let resp = local_state.load_chat_list(start_index, stop_index).await?;
            send(write.clone(), resp).await?;
        }
        Request::LoadMessageList {
            start_index,
            stop_index,
        } => {
            let resp = local_state
                .load_message_list(start_index, stop_index)
                .await?;
            send(write.clone(), resp).await?;
        }
        Request::SelectAccount { account } => {
            info!("selecting account {}", account);
            let resp = local_state.select_account(account).await?;
            send(write.clone(), resp).await?;
        }
        Request::SendTextMessage { text } => {
            local_state.send_text_message(text).await?;
            local_state.send_update(write.clone()).await?;
        }
        Request::SendFileMessage {
            typ,
            path,
            text,
            mime,
        } => {
            local_state.send_file_message(typ, path, text, mime).await?;
            local_state.send_update(write.clone()).await?;
        }
        Request::MaybeNetwork => {
            info!("maybe network");

            local_state.maybe_network().await?;
        }
    }
    Ok(())
}
