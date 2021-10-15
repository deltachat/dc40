#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use async_std::net::{TcpListener, TcpStream};
use async_std::sync::{Arc, RwLock};
use async_std::task;
use async_tungstenite::tungstenite::{Error, Message};
use dc40_backend::{commands, state::*};
use futures::StreamExt;
use log::{info, warn};
use shared::*;

fn main() {
    femme::with_level(log::LevelFilter::Info);

    let local_state = task::block_on(async {
        match LocalState::new().await {
            Ok(local_state) => local_state,
            Err(err) => panic!("Can't restore local state: {}", err),
        }
        //.expect(format!("Local state could not be restored: {}", err))
    });

    let local_state_clone = local_state.clone();
    std::thread::spawn(|| {
        let addr = "127.0.0.1:8081";

        task::block_on(async move {
            // Create the event loop and TCP listener we'll accept connections on.
            let try_socket = TcpListener::bind(&addr).await;
            let listener = try_socket.expect("Failed to bind");
            info!("Listening on: {}", addr);

            while let Ok((stream, _)) = listener.accept().await {
                let local_state = local_state_clone.clone();

                task::spawn(async move {
                    if let Err(err) = accept_connection(stream, local_state).await {
                        if let Some(err) = err.downcast_ref::<Error>() {
                            match err {
                                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {}
                                err => warn!("Error processing connection: {:?}", err),
                            }
                        } else {
                            warn!("Error processing connection: {:?}", err);
                        }
                    }
                });
            }
        });
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::load_backup])
        .manage(local_state)
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
        info!("request: {:?}", &parsed);
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
    writer: Arc<RwLock<T>>,
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

            local_state.login(id, &ctx, &email, &password).await?;

            local_state.send_account_details(id, writer.clone()).await?;
        }

        Request::SelectChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.select_chat(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
            send(writer.clone(), resp).await?;
        }
        Request::PinChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.pin_chat(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
            send(writer.clone(), resp).await?;
        }
        Request::UnpinChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.unpin_chat(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
            send(writer.clone(), resp).await?;
        }
        Request::ArchiveChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.archive_chat(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
            send(writer.clone(), resp).await?;
        }
        Request::UnarchiveChat {
            account: id,
            chat_id,
        } => {
            let resp = local_state.unarchive_chat(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
            send(writer.clone(), resp).await?;
        }
        Request::LoadChatList {
            start_index,
            stop_index,
        } => {
            let resp = local_state.load_chat_list(start_index, stop_index).await?;
            send(writer.clone(), resp).await?;
        }
        Request::LoadMessageList {
            start_index,
            stop_index,
        } => {
            let resp = local_state
                .load_message_list(Some((start_index, stop_index)))
                .await?;
            send(writer.clone(), resp).await?;
        }
        Request::SelectAccount { account } => {
            info!("selecting account {}", account);
            let resp = local_state.select_account(account).await?;
            send(writer.clone(), resp).await?;

            // TODO: store indicies
            let resp = local_state.load_message_list(None).await?;
            send(writer.clone(), resp).await?;
        }
        Request::SendTextMessage { text } => {
            local_state.send_text_message(text).await?;
            local_state.send_update(writer.clone()).await?;
        }
        Request::SendFileMessage {
            typ,
            path,
            text,
            mime,
        } => {
            local_state.send_file_message(typ, path, text, mime).await?;
            local_state.send_update(writer.clone()).await?;
        }
        Request::MaybeNetwork => {
            info!("maybe network");
            local_state.maybe_network().await?;
        }
        Request::AcceptContactRequest {
            account: id,
            chat_id,
        } => {
            local_state.accept_contact_request(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
        }
        Request::BlockContact {
            account: id,
            chat_id,
        } => {
            local_state.block_contact(id, chat_id).await?;
            local_state.send_update(writer.clone()).await?;
        }
        Request::GetAccountDetail { id } => {
            local_state.send_account_details(id, writer).await?;
        }
        Request::GetContacts => local_state.send_contacts(writer).await?,
    }
    Ok(())
}
