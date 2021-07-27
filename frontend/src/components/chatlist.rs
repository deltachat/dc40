use shared::ChatState;
use std::rc::Rc;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

use super::list::List;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub selected_account: Irc<Option<String>>,
    pub selected_chat_id: Irc<Option<u32>>,
    pub selected_chat: Irc<Option<ChatState>>,
    pub selected_chat_length: Irc<usize>,
    pub select_chat_callback: Callback<(String, u32)>,
    pub chats: Irc<Vec<ChatState>>,
    pub chats_range: Irc<(usize, usize)>,
    pub chats_len: Irc<usize>,
    pub fetch_callback: Callback<(usize, usize)>,
}

pub struct Chatlist {
    props: Props,
    link: ComponentLink<Self>,
}

impl Component for Chatlist {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Chatlist { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let account = self
            .props
            .selected_account
            .clone_inner()
            .unwrap_or_default();
        let cb = self.props.select_chat_callback.clone();
        let selected_chat_id = self.props.selected_chat_id.clone();

        let render_element: Rc<dyn Fn(ChatState) -> Html> =
            Rc::new(move |chat: ChatState| -> Html {
                let chat_id = chat.id;
                let cb = cb.clone();
                let account = account.clone();
                let callback: Callback<_> = (move |_| cb.emit((account.clone(), chat_id))).into();

                html! {
                    <Chat
                     chat=chat.clone()
                     selected_chat_id=selected_chat_id.clone()
                     select_callback=callback />
                }
            });

        html! {
            <div class="chats">
                <div class="account-header">
                    <div class="account-info">
                        {self.props.selected_account.clone_inner().unwrap_or_default()}
                    </div>
                </div>
                <List<ChatState>
                    class="chat-list".to_string()
                    list=self.props.chats.clone()
                    list_range = self.props.chats_range.clone()
                    list_len=self.props.chats_len.clone()
                    selected_id=self.props.selected_chat_id.clone()
                    fetch_callback=self.props.fetch_callback.clone()
                    render_element=render_element
                    auto_scroll=false
                    batch_size=10 />
              </div>
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct ChatProps {
    pub chat: ChatState,
    pub selected_chat_id: Irc<Option<u32>>,
    pub select_callback: Callback<()>,
}

struct Chat {
    props: ChatProps,
    link: ComponentLink<Self>,
}

impl Component for Chat {
    type Message = ();
    type Properties = ChatProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Chat { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let chat = &self.props.chat;
        let badge = if chat.fresh_msg_cnt > 0 {
            html! {
                <div class="chat-badge-bubble">{chat.fresh_msg_cnt}</div>
            }
        } else {
            html! {}
        };
        let image_style = format!("background-color: #{:06X}", chat.color);
        let image = if let Some(ref profile_image) = chat.profile_image {
            let src = format!("asset://{}", profile_image.to_string_lossy());

            html! {
                <img
                   class="image-icon"
                   src=src
                   alt="chat avatar" />
            }
        } else {
            html! {
                <div class="letter-icon" style={image_style}>
                    {chat.name.chars().next().unwrap_or_default()}
                </div>
            }
        };

        let mut class_name = "chat-list-item".to_string();
        if &*self.props.selected_chat_id == &Some(chat.id) {
            class_name += " active";
        }

        let cb = self.props.select_callback.clone();
        let onclick: Callback<_> = (move |_| cb.emit(())).into();

        html! {
            <div class=class_name onclick=onclick key=chat.id>
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
    }
}
