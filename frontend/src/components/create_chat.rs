use std::{collections::HashSet, iter::FromIterator};

use itertools::Itertools;
use log::info;
use shared::ContactInfo;
use web_sys::HtmlInputElement;
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
    query: String,
    group_name_input_ref: NodeRef,
}

pub enum Msg {
    Toggle(u32),
    Send,
    OnInputQuery(String),
}

impl Component for CreateChat {
    type Message = Msg;
    type Properties = Props;

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        info!("contacts: {:?}", *props.contacts);
        if props.contacts.is_none() {
            self.props.contact_cb.emit(());
        }
        self.props.neq_assign(props)
    }

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        if props.contacts.is_none() {
            info!("requesting contacts");
            props.contact_cb.emit(());
        }
        CreateChat {
            props,
            link,
            selected: HashSet::new(),
            query: String::new(),
            group_name_input_ref: NodeRef::default(),
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
                    let name = self
                        .group_name_input_ref
                        .cast::<HtmlInputElement>()
                        .unwrap()
                        .value();
                    if !name.is_empty() {
                        self.props
                            .create_group_chat_cb
                            .emit((self.selected.clone(), name));
                    }
                }
                self.props.add_chat_close_cb.emit(());
                true
            }
            Msg::OnInputQuery(change) => {
                self.query = change;
                true
            }
        }
    }

    fn view(&self) -> Html {
        let cb = self.link.callback(|id| Msg::Toggle(id));
        let send = self.link.callback(|_| Msg::Send);
        let contacts = if let Some(contacts) = &*self.props.contacts {
            let chars = self.query.chars().unique().collect_vec();
            let filtered_contacts: Box<dyn Iterator<Item = &ContactInfo>> =
                if !self.query.is_empty() {
                    Box::new(contacts.iter().filter(|a| {
                        let mail_chars = HashSet::<char>::from_iter(a.mail.chars());
                        let mut mail_ok = true;
                        let name_chars = HashSet::<char>::from_iter(a.display_name.chars());
                        let mut name_ok = true;

                        for chara in &chars {
                            if !mail_chars.contains(&chara) {
                                mail_ok = false
                            }
                            if !name_chars.contains(&chara) {
                                name_ok = false
                            }
                        }
                        mail_ok || name_ok
                    }))
                } else {
                    Box::new(contacts.iter())
                };
            html!({for filtered_contacts.map(move |contact|{
                let cb_clone = cb.clone();
                let id = contact.id.clone();
                let toggle_contact_cb: Callback<_> = (move |_| cb_clone.emit(id)).into();
                html!(
                <div key=contact.mail.clone() onclick=toggle_contact_cb class=classes!("contact", (self.selected.contains(&contact.id)).then(|| "selected"))>
                    <h2>{contact.display_name.clone()}</h2>
                    <p>{contact.mail.clone()}</p>
                </div>
            )})})
        } else {
            html!(<p class="text-center"> {"No contacts"}</p>)
        };

        let cb = self.props.add_chat_close_cb.clone();
        let close_cb: Callback<_> = (move |_| cb.emit(())).into();

        let on_search_input = self
            .link
            .callback(|e: InputData| Msg::OnInputQuery(e.value));

        html! {
            <div class="create-chat">
                <div class="search">
                    <button id="close" onclick=close_cb> <div class="icon arrow-back" /> </button>
                    <input size="1" oninput=on_search_input id="search-bar" type="text" />
                    <button id="create-chat-button" onclick=send> <div class=classes!("icon", "send", "small", if self.selected.len() != 0 {"ok"} else {"err"}) /> </button>
                </div>

                <div class=classes!( if self.selected.len() > 1 {"open"} else {"closed"}, "wrapper") >
                    <div class="group-name">
                        <label for="search-bar">{"Group-name: "}</label>
                        <input ref=self.group_name_input_ref.clone() size="1" alt="Group name"/>
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
