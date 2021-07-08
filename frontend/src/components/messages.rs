use shared::*;
use std::rc::Rc;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

use crate::components::list::List;
use crate::components::message::Message;

use log::info;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub messages: Irc<Vec<ChatMessage>>,
    pub messages_range: Irc<(usize, usize)>,
    pub messages_len: Irc<usize>,
    pub selected_chat_id: Irc<Option<u32>>,
    pub fetch_callback: Callback<(usize, usize)>,
}

pub struct Messages {
    props: Props,
    link: ComponentLink<Messages>,
}

impl Component for Messages {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Messages { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        info!("Update Message list{:?}", _msg);
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let render_element: Rc<dyn Fn(ChatMessage) -> Html> =
            Rc::new(move |msg: ChatMessage| -> Html {
                html! { <Message message=msg /> }
            });
        info!("messages {:?}", self.props.messages.len());
        if (self.props.messages.len() > 0) {
            html! {
              <List<ChatMessage>
                 class="message-list".to_string()
                 list=self.props.messages.clone()
                 list_range = self.props.messages_range.clone()
                 list_len=self.props.messages_len.clone()
                 selected_id=self.props.selected_chat_id.clone()
                 fetch_callback=self.props.fetch_callback.clone()
                 render_element=render_element
                 auto_scroll=true
                 batch_size=30 />
            }
        } else {
            html! {
              <div>{ "No messages "}</div>
            }
        }
    }
}
