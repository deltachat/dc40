use std::collections::HashSet;

use log::info;
use shared::ContactInfo;
use yew::prelude::*;
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub contacts: Irc<Option<Vec<ContactInfo>>>,
    pub contact_cb: Callback<()>,
}

pub struct CreateChat {
    link: ComponentLink<Self>,
    props: Props,
    selected: HashSet<u32>,
}

pub enum Msg {
    Toggle(u32),
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
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        info!("new props: {:?}", props.contacts);
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let cb = self.link.callback(|id| Msg::Toggle(id));

        let contacts = if let Some(contacts) = &*self.props.contacts {
            html!({for contacts.iter().map(move |contact|{
                let cb_clone = cb.clone();
                let id = contact.id.clone();
                let toggle_contact_cb: Callback<_> = (move |_| cb_clone.emit(id)).into();
                html!(
                <div onclick=toggle_contact_cb class=classes!("contact", (self.selected.contains(&contact.id)).then(|| "selected"))>
                    <h1>{contact.display_name.clone()}</h1>
                    <p>{contact.mail.clone()}</p>
                </div>
            )})})
        } else {
            html!(<p> {"No contacts"}</p>)
        };
        html! {
            <div class="create-chat">
                <div class="search">
                    <div contenteditable="true" type="text" class="search-bar">{"User..."} </div>
                    <button class="create-chat-button"> {"+"} </button>
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
