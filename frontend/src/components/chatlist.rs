use shared::{ChatState, SharedAccountState};
use std::collections::HashMap;
use std::rc::Rc;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

use super::context_menu::ContextMenu;
use super::list::List;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub selected_account: Irc<Option<u32>>,
    pub selected_account_details: Option<SharedAccountState>,
    pub selected_chat_id: Irc<Option<u32>>,
    pub selected_chat: Irc<Option<ChatState>>,
    pub selected_chat_length: Irc<usize>,
    pub select_chat_callback: Callback<(u32, u32)>,
    pub pin_chat_callback: Callback<(u32, u32)>,
    pub unpin_chat_callback: Callback<(u32, u32)>,
    pub archive_chat_callback: Callback<(u32, u32)>,
    pub unarchive_chat_callback: Callback<(u32, u32)>,
    pub chats: Irc<Vec<ChatState>>,
    pub chats_range: Irc<(usize, usize)>,
    pub chats_len: Irc<usize>,
    pub fetch_callback: Callback<(usize, usize)>,
    pub create_chat_callback: Callback<()>,
}

pub struct Chatlist {
    props: Props,
}

impl Component for Chatlist {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Chatlist { props }
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
        let pin_cb = self.props.pin_chat_callback.clone();
        let unpin_cb = self.props.unpin_chat_callback.clone();
        let archive_cb = self.props.archive_chat_callback.clone();
        let unarchive_cb = self.props.unarchive_chat_callback.clone();

        let selected_chat_id = self.props.selected_chat_id.clone();

        let render_element: Rc<dyn Fn(ChatState) -> Html> =
            Rc::new(move |chat: ChatState| -> Html {
                let chat_id = chat.id;
                let cb = cb.clone();
                let account = account.clone();
                let callback: Callback<_> = (move |_| cb.emit((account.clone(), chat_id))).into();

                let mut actions = HashMap::new();
                if chat.is_pinned {
                    let unpin_cb = unpin_cb.clone();
                    let unpin_callback: Callback<()> =
                        (move |_| unpin_cb.emit((account.clone(), chat_id))).into();

                    actions.insert("Unpin".to_string(), unpin_callback);
                } else {
                    let pin_cb = pin_cb.clone();
                    let pin_callback: Callback<()> =
                        (move |_| pin_cb.emit((account.clone(), chat_id))).into();

                    actions.insert("Pin".to_string(), pin_callback);
                }

                if chat.is_archived {
                    let unarchive_cb = unarchive_cb.clone();
                    let unarchive_callback: Callback<()> =
                        (move |_| unarchive_cb.emit((account.clone(), chat_id))).into();

                    actions.insert("Unarchive".to_string(), unarchive_callback);
                } else {
                    let archive_cb = archive_cb.clone();
                    let archive_callback: Callback<()> =
                        (move |_| archive_cb.emit((account.clone(), chat_id))).into();

                    actions.insert("Archive".to_string(), archive_callback);
                }

                html! {
                    <ContextMenu actions=actions>
                      <Chat
                        chat=chat.clone()
                        selected_chat_id=selected_chat_id.clone()
                        select_callback=callback />
                    </ContextMenu>
                }
            });

        let mut email = html! {};
        let mut name = html! {};
        if let Some(ref details) = self.props.selected_account_details {
            if let Some(ref display_name) = details.display_name {
                name = html! {
                    <div>{display_name}</div>
                };
            }

            email = html! {
                <div>{details.email.clone()}</div>
            };
        };

        let create_chat_cb = self.props.create_chat_callback.clone();
        let onclick: Callback<_> = (move |_| create_chat_cb.emit(())).into();

        html! {
            <div class="chats">
                <div class="account-header">
                    <div class="account-info">
                        {name}
                        {email}
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
            <button id="new_chat_button" onclick=onclick> {"+"} </button>
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
}

impl Component for Chat {
    type Message = ();
    type Properties = ChatProps;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Chat { props }
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
