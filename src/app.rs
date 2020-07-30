use std::collections::HashMap;

use anyhow::Error;
use log::*;
use yew::format::Json;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

use shared::*;

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
}

impl App {
    fn view_data(&self) -> Html {
        let link = self.link.clone();
        if let Some(ref state) = self.data {
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

                    html! {
                        <div class="chat-list-item" onclick=callback key=chat.id>
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
                    <Messages messages=state.messages.clone() />
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

#[derive(Properties, Clone, PartialEq)]
struct MessagesProps {
    messages: HashMap<String, ChatMessage>,
}

struct Messages {
    messages: HashMap<String, ChatMessage>,
    link: ComponentLink<Messages>,
    messages_ref: NodeRef,
}

impl Messages {
    fn scroll_to_bottom(&self) {
        if let Some(el) = self.messages_ref.cast::<web_sys::Element>() {
            let mut opts = web_sys::ScrollToOptions::new();
            let scroll_height = el.scroll_height();
            opts.top(scroll_height as f64);
            el.scroll_to_with_scroll_to_options(&opts);
        }
    }
}

impl Component for Messages {
    type Message = ();
    type Properties = MessagesProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Messages {
            messages: props.messages,
            link,
            messages_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.messages != props.messages {
            self.messages = props.messages;
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        self.scroll_to_bottom();
    }

    fn view(&self) -> Html {
        html! {
            <div class="message-list" ref=self.messages_ref.clone()>
            { self.messages.iter().map(|(key, msg)| {
                html! {
                    <Message message=msg />
                }
            }).collect::<Html>() }
            </div>
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
struct MessageProps {
    message: ChatMessage,
}

struct Message {
    message: ChatMessage,
    link: ComponentLink<Message>,
}

impl Component for Message {
    type Message = ();
    type Properties = MessageProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Message {
            message: props.message,
            link,
        }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let msg = &self.message;
        let image_style = format!("background-color: #{}", msg.from_color);
        let image = if let Some(ref profile_image) = msg.from_profile_image {
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
                   {msg.from_first_name.chars().next().unwrap()}
                </div>
            }
        };

        html! {
            <div class="message" key=msg.id>
                <div class="message-text">
                <div class="message-icon">{image}</div>
                <div class="message-body">
                <div class="message-header">
                  <div class="message-sender">{&msg.from_first_name}</div>
                  <div class="message-timestamp">
                    {msg.timestamp}
                  </div>
                  </div>
                  <div class="message-inner-text">
                    {msg.text.as_ref().cloned().unwrap_or_default()}
                  </div>
                </div>
                </div>
            </div>
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
