use log::*;
use yew::{html, Callback, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};
use yewtil::NeqAssign;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub send_callback: Callback<String>,
}

pub struct MessageInput {
    input_ref: NodeRef,
    props: Props,
    link: ComponentLink<Self>,
}

impl MessageInput {
    fn input(&self) -> Option<web_sys::HtmlInputElement> {
        self.input_ref.cast::<web_sys::HtmlInputElement>()
    }
}

pub enum Msg {
    OnChange(yew::ChangeData),
}

impl Component for MessageInput {
    type Properties = Props;
    type Message = Msg;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        MessageInput {
            props,
            link,
            input_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::OnChange(change) => {
                info!("chat message: {:?}", change);
                if let yew::ChangeData::Value(text) = change {
                    if !text.trim().is_empty() {
                        if let Some(ref input) = self.input() {
                            input.set_value("");
                        }
                        self.props.send_callback.emit(text);
                    }
                }
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let onchange = self.link.callback(Msg::OnChange);

        html! {
            <div class="chat-input">
                <input
                  type="text"
                  placeholder="Send a message"
                  onchange=onchange ref=self.input_ref.clone() />
            </div>
        }
    }
}
