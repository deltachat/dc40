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
                     selected_chat_id=state.selected_chat_id />
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

#[derive(Properties, Clone, PartialEq)]
struct MessagesProps {
    messages: Vec<ChatMessage>,
    messages_len: usize,
    messages_range: (usize, usize),
    selected_chat_id: Option<u32>,
}

struct Messages {
    props: MessagesProps,
    link: ComponentLink<Messages>,
    messages_ref: NodeRef,
    scroll_bottom_next: bool,
    scroll: (i32, i32),
}

impl Messages {
    fn messages_div(&self) -> web_sys::Element {
        self.messages_ref.cast::<web_sys::Element>().unwrap()
    }

    fn scroll_to_bottom(&mut self) {
        let el = self.messages_div();
        let mut opts = web_sys::ScrollToOptions::new();
        let scroll_height = el.scroll_height();
        opts.top(scroll_height as f64);
        el.scroll_to_with_scroll_to_options(&opts);
        self.scroll_bottom_next = false;
    }

    fn scroll_to_last(&self) {
        info!("scroll to last");
        let el = self.messages_div();

        let new_scroll = el.scroll_height() - el.client_height();
        el.set_scroll_top(self.scroll.0 + (new_scroll - self.scroll.1));
    }

    fn send_app_message(&self, msg: Msg) {
        let p = self.link.get_parent().expect("missing parent");
        let parent = p.clone().downcast::<App>();
        info!("sending {:?}", msg);

        parent.send_message(msg);
    }
}

enum MessagesMessage {
    OnScroll(web_sys::Event),
}

impl Component for Messages {
    type Message = MessagesMessage;
    type Properties = MessagesProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Messages {
            props,
            link,
            messages_ref: NodeRef::default(),
            scroll_bottom_next: true,
            scroll: (0, 0),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            MessagesMessage::OnScroll(ev) => {
                let el = self.messages_div();

                let from_top = el.scroll_top();
                let from_bottom = el.scroll_height() - el.scroll_top();
                self.scroll = (el.scroll_top(), el.scroll_height() - el.client_height());

                // element.scrollHeight - element.scrollTop === element.clientHeight
                let is_end = el.scroll_height() - el.scroll_top() == el.client_height();

                let MessagesProps {
                    messages_len,
                    messages_range,
                    ..
                } = self.props;

                if from_top < 20 && messages_range.0 > 0 {
                    info!("Load more (top) {}", from_top);

                    let start_index = if messages_range.0 < 20 {
                        0
                    } else {
                        messages_range.0.saturating_sub(20)
                    };
                    let len = (messages_range.1.saturating_sub(start_index)).max(50);
                    let stop_index = (start_index + len).min(messages_len);

                    self.send_app_message(Msg::WsRequest(Request::LoadMessageList {
                        start_index,
                        stop_index,
                    }));
                } else if is_end {
                    info!(
                        "Load more (bottom) {} ({:?}, {})",
                        from_bottom, messages_range, messages_len
                    );

                    // let stop_index = if messages_len.saturating_sub(messages_range.1) < 20 {
                    //     messages_len
                    // } else {
                    //     messages_range.1 + 20
                    // };
                    // let len = (stop_index.saturating_sub(messages_range.0)).max(50);
                    // let start_index = stop_index.saturating_sub(len);

                    // self.send_app_message(Msg::WsRequest(Request::LoadMessageList {
                    //     start_index,
                    //     stop_index,
                    // }));
                }

                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.scroll_bottom_next = self.props.selected_chat_id != props.selected_chat_id;
            self.props = props;
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if self.scroll_bottom_next || first_render {
            self.scroll_to_bottom();
        } else {
            self.scroll_to_last();
        }
    }

    fn view(&self) -> Html {
        let onscroll = self.link.callback(MessagesMessage::OnScroll);

        html! {
            <div class="message-list" ref=self.messages_ref.clone() onscroll=onscroll>
            { self.props.messages.iter().map(|msg| {
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

        let file = match msg.viewtype {
            Viewtype::Image | Viewtype::Gif => {
                if let Some(ref file) = msg.file {
                    info!("{}, {}", msg.file_height, msg.file_width);
                    html! {
                        <div className="message-image">
                          <img
                            src={format!("dc://{}", file.display())}
                            alt="image"
                            height=300
                            width="auto" />
                        </div>
                    }
                } else {
                    html! {}
                }
            }
            _ => html! {},
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
                  { file }
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
