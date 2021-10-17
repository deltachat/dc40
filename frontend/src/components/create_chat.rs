use std::collections::HashSet;

use log::info;
use shared::ContactInfo;
use yew::prelude::*;
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub contacts: Irc<Option<Vec<ContactInfo>>>,
    pub contact_cb: Callback<()>,
    pub create_chat_cb: Callback<HashSet<u32>>,
    pub create_group_chat_cb: Callback<(HashSet<u32>, String)>,
    pub add_chat_close_cb: Callback<()>,
}

pub struct CreateChat {
    link: ComponentLink<Self>,
    props: Props,
    selected: HashSet<u32>,
    group_name: String,
}

pub enum Msg {
    Toggle(u32),
    Send,
    OnChange(yew::ChangeData),
}

impl Component for CreateChat {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        if *props.contacts == None {
            props.contact_cb.emit(());
        }
        CreateChat {
            props,
            link,
            selected: HashSet::new(),
            group_name: String::from("generic group"),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Toggle(id) => {
                if self.selected.contains(&id) {
                    self.selected.remove(&id);
                } else {
                    self.selected.insert(id);
                }
                true
            }
            Msg::Send => {
                if self.selected.len() == 1 {
                    info!("creating new 1o1 chat");
                    self.props.create_chat_cb.emit(self.selected.clone());
                } else if self.selected.len() > 1 {
                    info!("creating new group chat with users: {:?}", self.selected);
                    self.props
                        .create_group_chat_cb
                        .emit((self.selected.clone(), self.group_name.clone()));
                }
                self.props.add_chat_close_cb.emit(());
                true
            }
            Msg::OnChange(change) => {
                if let yew::ChangeData::Value(text) = change {
                    if !text.trim().is_empty() {
                        self.link.send_message(Msg::Send);
                    }
                }
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        info!("new props: {:?}", props.contacts);
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let cb = self.link.callback(|id| Msg::Toggle(id));
        let send = self.link.callback(|_| Msg::Send);
        let contacts = if let Some(contacts) = &*self.props.contacts {
            html!({for contacts.iter().map(move |contact|{
                let cb_clone = cb.clone();
                let id = contact.id.clone();
                let toggle_contact_cb: Callback<_> = (move |_| cb_clone.emit(id)).into();
                html!(
                <div onclick=toggle_contact_cb class=classes!("contact", (self.selected.contains(&contact.id)).then(|| "selected"))>
                    <h2>{contact.display_name.clone()}</h2>
                    <p>{contact.mail.clone()}</p>
                </div>
            )})})
        } else {
            html!(<p class="text-center"> {"No contacts"}</p>)
        };

        let cb = self.props.add_chat_close_cb.clone();
        let close_cb: Callback<_> = (move |_| cb.emit(())).into();

        html! {
            <div class="create-chat">
                <div class="search">
                    <button id="close" onclick=close_cb> <div class="icon arrow-back" /> </button>
                    <input size="1" id="search-bar" type="text" />
                    <button id="create-chat-button" onclick=send> <div class=classes!("icon", "send", "small", if self.selected.len() != 0 {"ok"} else {"err"}) /> </button>
                </div>

                <div class=classes!( if self.selected.len() > 1 {"open"} else {"closed"}, "wrapper") >
                    <div class="group-name">
                        <label for="search-bar">{"Group-name: "}</label>
                        <input size="1" alt="Group name"/>
                    </div>
                </div>

                <div class="contact-list">
                    <div>
                        {contacts}
                    </div>
                </div>
            </div>
        }
    }
}
