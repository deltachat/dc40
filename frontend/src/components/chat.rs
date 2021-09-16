use crate::components::{message_input::MessageInput, messages::Messages};
use shared::ChatState;
use yew::prelude::*;
use yewtil::{ptr::Mrc, NeqAssign};

use super::messages::Props as MessagesProps;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub accept_contact_request_callback: Callback<()>,
    pub block_contact_callback: Callback<()>,
    pub send_message: Callback<String>,
    pub messages_props: MessagesProps,
    pub selected_chat: Mrc<Option<ChatState>>,
}

pub struct Chat {
    props: Props,
}

pub enum Msg {}

impl Component for Chat {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        // callbacks
        let block_contact_callback = self.props.block_contact_callback.clone();
        let block_contact_callback: Callback<_> = (move |_| block_contact_callback.emit(())).into();
        let accept_contact_request_callback = self.props.accept_contact_request_callback.clone();
        let accept_contact_request_callback: Callback<_> =
            (move |_| accept_contact_request_callback.emit(())).into();

        let chat = self.props.selected_chat.as_ref().as_ref().unwrap();

        // toggle between sending messages and accepting chat
        let input = if chat.is_contact_request {
            html! {
            <div class="contact-request-buttons">
                <button class="block-button" onclick=block_contact_callback>
                {"Block"}
                </button>
                <button class="accept-button" onclick=accept_contact_request_callback>
                {"Accept"}
                </button>
            </div>
            }
        } else {
            html! {
                <MessageInput send_callback=self.props.send_message.clone() />
            }
        };

        let (title, subtitle) = get_titles(&chat);

        html! {
            <div class="chat">
                <div class="chat-header">
                    <div>
                        <div class="chat-header-name">{title}</div>
                        <div class="chat-header-subtitle">
                        { subtitle }
                        </div>
                    </div>
                </div>

                <Messages with self.props.messages_props.clone() />
                { input }
            </div>

        }
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

        let subtitle = if chat.is_contact_request {
            "Contact Request".to_string()
        } else if chat.chat_type == "Group" || chat.chat_type == "VerifiedGroup" {
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
