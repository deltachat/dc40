use anyhow::Error;
use log::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{format::Bincode, props};
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yewtil::{
    ptr::{Irc, Mrc},
    NeqAssign,
};

use shared::*;

use crate::components::{
    chat::Chat,
    chatlist::Chatlist,
    messages::Props as MessagesProps,
    modal::Modal,
    sidebar::Sidebar,
    windowmanager::{Props as FileManagerProps, WindowManager},
};

#[derive(Debug)]
pub enum WsAction {
    Connect,
    Disconnect,
    Lost,
}

#[derive(Debug)]
pub enum Msg {
    Connected,
    WsAction(WsAction),
    WsReady(Result<Response, Error>),
    WsRequest(Request),
    ShowAccountCreation,
    CancelAccountCreation,
    AccountCreation(String, String),
}

impl From<WsAction> for Msg {
    fn from(action: WsAction) -> Self {
        Msg::WsAction(action)
    }
}

pub struct App {
    link: ComponentLink<App>,
    model: Model,
    ws: Option<WebSocketTask>,
}

#[derive(Debug, Clone, Default)]
struct Model {
    accounts: Mrc<HashMap<u32, SharedAccountState>>,
    errors: Mrc<Vec<String>>,
    selected_account: Mrc<Option<u32>>,
    selected_chat_id: Mrc<Option<u32>>,
    selected_chat: Mrc<Option<ChatState>>,
    selected_chat_length: Mrc<usize>,
    chats: Mrc<Vec<ChatState>>,
    chats_range: Mrc<(usize, usize)>,
    chats_len: Mrc<usize>,
    messages_range: Mrc<(usize, usize)>,
    message_items: Mrc<Vec<ChatItem>>,
    messages: Mrc<Vec<ChatMessage>>,
    show_account_creation: bool,
}

impl App {
    fn view_data(&self) -> Html {
        let link = self.link.clone();
        let send_message =
            link.callback(move |text| Msg::WsRequest(Request::SendTextMessage { text }));

        let chats_fetch_callback = link.callback(move |(start_index, stop_index)| {
            Msg::WsRequest(Request::LoadChatList {
                start_index,
                stop_index,
            })
        });
        let messages_fetch_callback = link.callback(move |(start_index, stop_index)| {
            Msg::WsRequest(Request::LoadMessageList {
                start_index,
                stop_index,
            })
        });
        let select_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::SelectChat { account, chat_id })
        });

        let pin_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::PinChat { account, chat_id })
        });

        let unpin_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::UnpinChat { account, chat_id })
        });
        let archive_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::ArchiveChat { account, chat_id })
        });

        let unarchive_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::UnarchiveChat { account, chat_id })
        });

        let create_account_callback = link.callback(move |_| Msg::ShowAccountCreation);
        let cancel_account_create_callback = link.callback(move |_| Msg::CancelAccountCreation);

        let submit_account_create_callback =
            link.callback(move |(email, password)| Msg::AccountCreation(email, password));

        let import_callback =
            link.callback(move |id| Msg::WsRequest(Request::GetAccountDetail { id }));

        let select_account_callback = link.callback(move |account| {
            info!("Account switched {}", account);
            Msg::WsRequest(Request::SelectAccount { account })
        });

        let account_creation_modal = if self.model.show_account_creation {
            html! {
                <Modal
                import_callback=import_callback
                 submit_callback=submit_account_create_callback
                 cancel_callback=cancel_account_create_callback />
            }
        } else {
            html! {}
        };

        let selected_account = self.model.selected_account.as_ref().unwrap_or_default();
        let account_details = self
            .model
            .accounts
            .get(&selected_account)
            .map(|s| s.clone());

        let messages = if let Some(chat) = &*self.model.selected_chat {
            let chat_id = chat.id;
            let accept_contact_request_callback = link.callback(move |_| {
                Msg::WsRequest(Request::AcceptContactRequest {
                    account: selected_account,
                    chat_id,
                })
            });
            let block_contact_callback = link.callback(move |_| {
                Msg::WsRequest(Request::BlockContact {
                    account: selected_account,
                    chat_id,
                })
            });
            let messages_props = props! {
                MessagesProps {
                    messages: self.model.messages.irc(),
                    messages_len: Irc::new(self.model.message_items.len()),
                    messages_range: self.model.messages_range.irc(),
                    selected_chat_id: self.model.selected_chat_id.irc(),
                    fetch_callback: messages_fetch_callback,

                }
            };

            html!(
                <Chat
                    accept_contact_request_callback=accept_contact_request_callback
                    block_contact_callback=block_contact_callback
                    send_message = send_message
                    messages_props = messages_props
                    selected_chat=self.model.selected_chat.clone()
                />
            )
        } else {
            html! {
              <div>{"No chat selected"}</div>
            }
        };

        let file_manager_props = props! {
            FileManagerProps {
                left: html!(
                    <div class="normal-panel">
                        <Sidebar
                        accounts=self.model.accounts.irc()
                        selected_account=self.model.selected_account.irc()
                        select_account_callback=select_account_callback
                        create_account_callback=create_account_callback
                        />
                    <Chatlist
                        selected_account=self.model.selected_account.irc()
                        selected_account_details=account_details
                        selected_chat_id=self.model.selected_chat_id.irc()
                        selected_chat=self.model.selected_chat.irc()
                        selected_chat_length=self.model.selected_chat_length.irc()
                        select_chat_callback=select_chat_callback
                        pin_chat_callback=pin_chat_callback
                        unpin_chat_callback=unpin_chat_callback
                        archive_chat_callback=archive_chat_callback
                        unarchive_chat_callback=unarchive_chat_callback
                        chats=self.model.chats.irc()
                        chats_range=self.model.chats_range.irc()
                        chats_len=self.model.chats_len.irc()
                        fetch_callback=chats_fetch_callback />
                    </div>),
                center: messages,
                right: None
            }
        };

        html! {
            <>
                {account_creation_modal}
                <WindowManager with file_manager_props/>
            </>
        }
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        link.send_message(WsAction::Connect);
        App {
            link,
            model: Model::default(),
            ws: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::WsAction(action) => match action {
                WsAction::Connect => {
                    let callback = self.link.callback(|Bincode(data)| Msg::WsReady(data));
                    let notification = self.link.callback(|status| match status {
                        WebSocketStatus::Opened => Msg::Connected,
                        WebSocketStatus::Closed | WebSocketStatus::Error => WsAction::Lost.into(),
                    });
                    let task = WebSocketService::connect_binary(
                        "ws://localhost:8081/",
                        callback,
                        notification,
                    )
                    .unwrap();
                    self.ws = Some(task);
                }
                WsAction::Disconnect => {
                    self.ws.take();
                }
                WsAction::Lost => {
                    self.ws = None;
                }
            },
            Msg::Connected => {
                let mut messages = vec![Msg::WsRequest(Request::LoadChatList {
                    start_index: 0,
                    stop_index: 10,
                })];

                if self.model.selected_chat.is_some() {
                    messages.push(Msg::WsRequest(Request::LoadMessageList {
                        start_index: 0,
                        stop_index: 0,
                    }));
                }

                self.link.send_message_batch(messages);
                return false;
            }
            Msg::WsReady(response) => match response {
                Ok(data) => match data {
                    Response::MessageList {
                        chat_id: _,
                        range,
                        items,
                        messages,
                    } => {
                        self.model.messages_range.neq_assign(range);
                        self.model.message_items.neq_assign(items);
                        self.model.messages.neq_assign(messages);

                        return true;
                    }
                    Response::ChatList { range, len, chats } => {
                        self.model.chats_range.neq_assign(range);
                        self.model.chats_len.neq_assign(len);
                        info!("ChatList {:?}", chats);
                        self.model.chats.neq_assign(chats);
                        return true;
                    }
                    Response::Account {
                        account,
                        chat,
                        chat_id,
                    } => {
                        self.model.selected_account.neq_assign(Some(account));
                        self.model.selected_chat.neq_assign(chat);
                        self.model.selected_chat_id.neq_assign(chat_id);

                        let message = Msg::WsRequest(Request::LoadChatList {
                            start_index: 0,
                            stop_index: 10,
                        });
                        self.link.send_message(message);
                        return true;
                    }
                    Response::RemoteUpdate { state } => {
                        info!("RemoteUpdate {:?}", state);
                        self.model.accounts.neq_assign(state.shared.accounts);
                        self.model.errors.neq_assign(state.shared.errors);
                        self.model
                            .selected_account
                            .neq_assign(state.shared.selected_account);
                        self.model
                            .selected_chat_id
                            .neq_assign(state.shared.selected_chat_id);
                        self.model
                            .selected_chat
                            .neq_assign(state.shared.selected_chat);
                        return true;
                    }
                    Response::Event { account, event } => {
                        match event {
                            Event::MessagesChanged { chat_id } => {
                                info!("changed {}", chat_id);
                                // refresh chat list
                                let mut messages = vec![Msg::WsRequest(Request::LoadChatList {
                                    start_index: self.model.chats_range.0,
                                    stop_index: self.model.chats_range.1,
                                })];

                                if *self.model.selected_chat_id.as_ref() == Some(chat_id) {
                                    // if the selected chat changed, refresh that
                                    messages.push(Msg::WsRequest(Request::LoadMessageList {
                                        start_index: self.model.messages_range.0,
                                        stop_index: self.model.messages_range.1,
                                    }));
                                }

                                self.link.send_message_batch(messages);
                            }
                            Event::MessageIncoming {
                                chat_id,
                                title,
                                body,
                            } => {
                                info!("incoming {}", chat_id);
                                let mut opts = web_sys::NotificationOptions::new();
                                opts.body(&body);
                                let notification =
                                    web_sys::Notification::new_with_options(&title, &opts).unwrap();
                                let onclick = wasm_bindgen::closure::Closure::wrap(Box::new(|| {
                                    info!("clicked notification");
                                })
                                    as Box<dyn Fn()>);
                                notification.set_onclick(Some(onclick.as_ref().unchecked_ref()));

                                // refresh chat list
                                let mut messages = vec![Msg::WsRequest(Request::LoadChatList {
                                    start_index: self.model.chats_range.0,
                                    stop_index: self.model.chats_range.1,
                                })];

                                if *self.model.selected_chat_id.as_ref() == Some(chat_id) {
                                    // if the selected chat changed, refresh that
                                    messages.push(Msg::WsRequest(Request::LoadMessageList {
                                        start_index: self.model.messages_range.0,
                                        stop_index: self.model.messages_range.1 + 1,
                                    }));
                                }

                                self.link.send_message_batch(messages);
                            }
                            Event::Log(log) => match log {
                                shared::Log::Info(_msg) => {
                                    // info!("[{}]: {:?}", account, msg);
                                }
                                shared::Log::Warning(msg) => {
                                    warn!("[{}]: {:?}", account, msg);
                                }
                                shared::Log::Error(msg) => {
                                    error!("[{}]: {:?}", account, msg);
                                }
                            },
                            _ => {}
                        }
                    }
                },
                Err(err) => {
                    warn!("{:#?}", err);
                }
            },
            Msg::WsRequest(req) => {
                if let Some(ws) = self.ws.as_mut() {
                    ws.send_binary(Bincode(&req));
                }
            }
            Msg::ShowAccountCreation => {
                self.model.show_account_creation = true;
                return true;
            }
            Msg::CancelAccountCreation => {
                self.model.show_account_creation = false;
                return true;
            }
            Msg::AccountCreation(email, password) => {
                let msg = Msg::WsRequest(Request::Login { email, password });
                self.link.send_message(msg);
                self.model.show_account_creation = false;
                return true;
            }
        }
        false
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        self.view_data()
    }
}
