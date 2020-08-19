use anyhow::Error;
use log::*;
use std::collections::HashMap;
use yew::format::Bincode;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};
use yewtil::{
    ptr::{Irc, Mrc},
    NeqAssign,
};

use shared::*;

use crate::components::{
    chatlist::Chatlist, message_input::MessageInput, messages::Messages, sidebar::Sidebar,
};

#[derive(Debug)]
pub enum WsAction {
    Connect,
    Disconnect,
    Lost,
}

#[derive(Debug)]
pub enum Msg {
    WsAction(WsAction),
    WsReady(Result<Response, Error>),
    Ignore,
    WsRequest(Request),
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
    messages_range: Mrc<(usize, usize)>,
    message_items: Mrc<Vec<ChatItem>>,
    messages: Mrc<Vec<ChatMessage>>,
}

impl App {
    fn view_data(&self) -> Html {
        let link = self.link.clone();
        let onsend = link.callback(move |text| Msg::WsRequest(Request::SendTextMessage { text }));

        let fetch_callback = link.callback(move |(start_index, stop_index)| {
            Msg::WsRequest(Request::LoadMessageList {
                start_index,
                stop_index,
            })
        });
        let select_chat_callback = link.callback(move |(account, chat_id)| {
            Msg::WsRequest(Request::SelectChat { account, chat_id })
        });

        html! {
            <>
                <Sidebar
                  accounts=self.model.accounts.irc()
                  selected_account=self.model.selected_account.irc()
                />
                <Chatlist
                  selected_account=self.model.selected_account.irc()
                  chats=self.model.chats.irc()
                  selected_chat_id=self.model.selected_chat_id.irc()
                  selected_chat=self.model.selected_chat.irc()
                  selected_chat_length =self.model.selected_chat_length.irc()
                  select_chat_callback=select_chat_callback
                />
                <div class="chat">
                  <div class="chat-header"> {
                    if let Some(chat) = &*self.model.selected_chat {
                        html! {
                            <div class="chat-header-name">{&chat.name}</div>
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
                   fetch_callback=fetch_callback />
                  <MessageInput send_callback=onsend />
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
                        WebSocketStatus::Opened => Msg::Ignore,
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
                    Response::RemoteUpdate { state } => {
                        self.model.accounts.neq_assign(state.shared.accounts);
                        self.model.errors.neq_assign(state.shared.errors);
                        self.model.chats.neq_assign(state.shared.chats);
                        self.model
                            .selected_account
                            .neq_assign(state.shared.selected_account);
                        self.model
                            .selected_chat_id
                            .neq_assign(state.shared.selected_chat_id);
                        self.model
                            .selected_chat
                            .neq_assign(state.shared.selected_chat);
                        self.model
                            .selected_chat_length
                            .neq_assign(state.shared.selected_chat_length);
                        return true;
                    }
                },
                Err(err) => {
                    warn!("{:?}", err);
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
        }
        false
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="app">
                { self.view_data() }
            </div>
        }
    }
}
