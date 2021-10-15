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
}

pub enum Msg {}

impl Component for CreateChat {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        if *props.contacts == None {
            props.contact_cb.emit(());
        }
        CreateChat { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        info!("new props: {:?}", props.contacts);
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let contacts = if let Some(contacts) = &*self.props.contacts {
            html!({for contacts.iter().map(|contact| html!(
                <div>
                    {contact.mail.clone()}
                    {contact.display_name.clone()}
                </div>
            ))})
        } else {
            html!(<p> {"No contacts"}</p>)
        };
        html! {
            <div class="create-chat">
                <div class="search">
                    <div contenteditable="true" type="text" class="search-bar">{"User..."} </div>
                    <button class="create-chat-button"> {"+"} </button>
                </div>

                <div class="ContactList">
                    {contacts}
                </div>
            </div>
        }
    }
}
