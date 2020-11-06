use anyhow::Error;
use log::*;
use std::collections::HashMap;
use yew::format::Bincode;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yewtil::{
    ptr::{Irc, Mrc},
    NeqAssign,
};

use shared::*;

use crate::components::{
    chatlist::Chatlist, message_input::MessageInput, messages::Messages, modal::Modal,
    sidebar::Sidebar,
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
    Ignore,
    WsRequest(Request),
    ShowAccountCreation,
    CancelAccountCreation,
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
    accounts: Mrc<HashMap<String, SharedAccountState>>,
    errors: Mrc<Vec<String>>,
    selected_account: Mrc<Option<String>>,
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
        let onsend = link.callback(move |text| Msg::WsRequest(Request::SendTextMessage { text }));

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

        let create_account_callback = link.callback(move |_| Msg::ShowAccountCreation);
        let cancel_account_create_callback = link.callback(move |_| Msg::CancelAccountCreation);

        let account_creation_modal = if self.model.show_account_creation {
            html! {
                <Modal cancel_callback=cancel_account_create_callback />
            }
        } else {
            html! {}
        };
        html! {
            <>
            { account_creation_modal }
              <div class="app">
                <Sidebar
                  accounts=self.model.accounts.irc()
                  selected_account=self.model.selected_account.irc()
                  create_account_callback=create_account_callback
                />
                <Chatlist
                  selected_account=self.model.selected_account.irc()
                  selected_chat_id=self.model.selected_chat_id.irc()
                  selected_chat=self.model.selected_chat.irc()
                  selected_chat_length =self.model.selected_chat_length.irc()
                  select_chat_callback=select_chat_callback
                  chats=self.model.chats.irc()
                  chats_range=self.model.chats_range.irc()
                  chats_len=self.model.chats_len.irc()
                  fetch_callback=chats_fetch_callback />
                <div class="chat">
                  <div class="chat-header"> {
                    if let Some(chat) = &*self.model.selected_chat {
                        let (title, subtitle) = get_titles(&chat);
                        html! {
                            <div>
                              <div class="chat-header-name">{title}</div>
                              <div class="chat-header-subtitle">
                                { subtitle }
                              </div>
                            </div>
                        }
                    } else {
                        html! {}
                    }
                  }
                  </div>

                  <Messages
                   messages=self.model.messages.irc()
                   messages_len=Irc::new(self.model.message_items.len())
                   messages_range=self.model.messages_range.irc()
                   selected_chat_id=self.model.selected_chat_id.irc()
                   fetch_callback=messages_fetch_callback />
                  <MessageInput send_callback=onsend />
                </div>
            </div>
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
                        "ws://localhost:8080/",
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
                        self.model.chats.neq_assign(chats);

                        return true;
                    }
                    Response::RemoteUpdate { state } => {
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
                            Event::MessageIncoming { chat_id } => {
                                info!("incoming {}", chat_id);
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
                                shared::Log::Info(msg) => {
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
            Msg::Ignore => {
                return false;
            }
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

/// Get the title and subtitle texts.
fn get_titles(chat: &ChatState) -> (String, String) {
    if chat.id == 1 {
        // deaddrop
        (
            "Contact Requests".to_string(),
            "Click message to start chatting".to_string(),
        )
    } else {
        let title = chat.name.to_string();

        let subtitle = if chat.chat_type == "Group" || chat.chat_type == "VerifiedGroup" {
            if chat.member_count == 1 {
                "1 member".to_string()
            } else {
                format!("{} members", chat.member_count)
            }
        } else if chat.is_self_talk {
            "Messages I sent to myself".to_string()
        } else if chat.is_device_talk {
            "Locally generated messages".to_string()
        } else {
            // TODO: print first member address
            "Private Chat".to_string()
        };

        (title, subtitle)
    }
}
