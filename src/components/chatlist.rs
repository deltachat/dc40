use shared::ChatState;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub selected_account: Irc<Option<String>>,
    pub chats: Irc<Vec<ChatState>>,
    pub selected_chat_id: Irc<Option<u32>>,
    pub selected_chat: Irc<Option<ChatState>>,
    pub selected_chat_length: Irc<usize>,
    pub select_chat_callback: Callback<(String, u32)>,
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
        html! {
            <div class="chats">
              <div class="account-header">
                <div class="account-info">
                  {self.props.selected_account.clone_inner().unwrap_or_default()}
                </div>
              </div>
              <div class="chat-list">
                { (&self.props.chats).iter().map(|chat| {
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
                    let account = self.props.selected_account.clone_inner().unwrap_or_default();
                    let cb = self.props.select_chat_callback.clone();
                    let chat_id = chat.id;
                    let callback: Callback<_> = (move |_| cb.emit((account.clone(), chat_id))).into();

                    let mut className = "chat-list-item".to_string();
                    if &*self.props.selected_chat_id == &Some(chat.id) {
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
        }
    }
}
