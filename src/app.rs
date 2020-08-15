use anyhow::Error;
use log::*;
use yew::format::Json;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

use shared::*;

use crate::components::messages::Messages;

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
    data: Option<SharedState>,
    ws: Option<WebSocketTask>,
    input_ref: NodeRef,
}

impl App {
    fn input(&self) -> Option<web_sys::HtmlInputElement> {
        self.input_ref.cast::<web_sys::HtmlInputElement>()
    }

    fn view_data(&self) -> Html {
        let link = self.link.clone();
        if let Some(ref state) = self.data {
            let input = self.input();
            let onchange = link.callback(move |change| {
                info!("chat message: {:?}", change);
                if let yew::ChangeData::Value(text) = change {
                    if !text.trim().is_empty() {
                        if let Some(ref input) = input {
                            input.set_value("");
                        }
                        Msg::WsRequest(Request::SendTextMessage { text })
                    } else {
                        Msg::Ignore
                    }
                } else {
                    Msg::Ignore
                }
            });

            let fetch_callback = link.callback(move |(start_index, stop_index)| {
                Msg::WsRequest(Request::LoadMessageList {
                    start_index,
                    stop_index,
                })
            });

            html! {
                <>
                  <div class="sidebar">
                    <div class="account-list">
                { state.accounts.iter().map(|(_, acc)| {
                    html! {
                        <div class="account">
                            <div class="letter-icon">
                              {acc.email.chars().next().unwrap()}
                            </div>
                        </div>
                    }
                }).collect::<Html>() }
                    </div>
                  </div>
                  <div class="chats">
                    <div class="account-header">
                      <div class="account-info">
                        {state.selected_account.as_ref().cloned().unwrap_or_default()}
                      </div>
                    </div>
                    <div class="chat-list">
                { state.chats.iter().map(|chat| {
                    let badge = if chat.fresh_msg_cnt > 0 {
                        html! {
                            <div class="chat-badge-bubble">{chat.fresh_msg_cnt}</div>
                        }
                    } else {
                        html! {}
                    };
                    let image_style = format!("background-color: #{}", chat.color);
                    let image = if let Some(ref profile_image) = chat.profile_image {
                        html! {
                            <img
                             class="image-icon"
                             src={format!("dc://{}", profile_image.to_string_lossy())}
                             alt="chat avatar"
                             />
                        }
                    } else {
                        html! {
                            <div class="letter-icon" style={image_style}>
                               {chat.name.chars().next().unwrap()}
                            </div>
                        }
                    };
                    let account = state.selected_account.as_ref().cloned().unwrap_or_default();
                    let chat_id = chat.id;
                    let callback = link.callback(move |_| {
                        Msg::WsRequest(Request::SelectChat {
                            account: account.clone(),
                            chat_id,
                        })
                    });
                    let mut className = "chat-list-item".to_string();
                    if state.selected_chat_id == Some(chat.id) {
                        className += " active";
                    }

                    html! {
                        <div class=className onclick=callback key=chat.id>
                            <div class="chat-icon">{image}</div>
                            <div class="chat-content">
                              <div class="chat-header">{&chat.name}</div>
                              <div class="chat-preview">{&chat.preview}</div>
                            </div>
                            <div class="chat-badge">
                            { badge }
                            </div>
                        </div>
                    }
                }).collect::<Html>() }
                   </div>
                 </div>
                    <div class="chat">
                    <div class="chat-header"> {
                        if let Some(ref chat) = state.selected_chat {
                            html! {
                                <div class="chat-header-name">{&chat.name}</div>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>
                    <Messages
                     messages=state.messages.clone()
                     messages_len=state.selected_messages_length
                     messages_range=state.selected_messages_range
                     selected_chat_id=state.selected_chat_id
                     fetch_callback=fetch_callback />
                    <div class="chat-input">
                      <input type="text" placeholder="Send a message" onchange=onchange ref=self.input_ref.clone() />
                    </div>
                 </div>
               </>
            }
        } else {
            html! {
                <p>{ "Data hasn't fetched yet." }</p>
            }
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
            data: None,
            ws: None,
            input_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::WsAction(action) => match action {
                WsAction::Connect => {
                    let callback = self.link.callback(|Json(data)| Msg::WsReady(data));
                    let notification = self.link.callback(|status| match status {
                        WebSocketStatus::Opened => Msg::Ignore,
                        WebSocketStatus::Closed | WebSocketStatus::Error => WsAction::Lost.into(),
                    });
                    let task =
                        WebSocketService::connect("ws://localhost:8080/", callback, notification)
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
            Msg::WsReady(response) => {
                self.data = response
                    .map(|data| match data {
                        Response::RemoteUpdate { state } => state.shared,
                    })
                    .map_err(|err| {
                        warn!("{:?}", err);
                        err
                    })
                    .ok();
            }
            Msg::Ignore => {
                return false;
            }
            Msg::WsRequest(req) => {
                if let Some(ws) = self.ws.as_mut() {
                    ws.send(Json(&req));
                }
            }
        }
        true
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
