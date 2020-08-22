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
    has_text: bool,
}

impl MessageInput {
    fn input(&self) -> Option<web_sys::HtmlInputElement> {
        self.input_ref.cast::<web_sys::HtmlInputElement>()
    }
}

pub enum Msg {
    OnChange(yew::ChangeData),
    OnInput(yew::InputData),
    Send,
}

impl Component for MessageInput {
    type Properties = Props;
    type Message = Msg;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        MessageInput {
            props,
            link,
            input_ref: NodeRef::default(),
            has_text: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Send => {
                if let Some(ref input) = self.input() {
                    let text = input.value();
                    if !text.is_empty() {
                        self.props.send_callback.emit(text);
                    }
                    input.set_value("");
                    self.has_text = false;
                }
                false
            }
            Msg::OnChange(change) => {
                info!("chat message: {:?}", change);
                if let yew::ChangeData::Value(text) = change {
                    if !text.trim().is_empty() {
                        self.link.send_message(Msg::Send);
                    }
                }
                false
            }
            Msg::OnInput(_) => {
                if let Some(ref input) = self.input() {
                    self.has_text = !input.value().is_empty();
                }
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let onchange = self.link.callback(Msg::OnChange);
        let oninput = self.link.callback(Msg::OnInput);
        let onclick = self.link.callback(|_| Msg::Send);

        let mut send_button_class = "send-button".to_string();
        if self.has_text {
            send_button_class += " active";
        }

        html! {
            <div class="chat-input">
                <input
                  type="text"
                  placeholder="Send a message"
                  onchange=onchange
                  oninput=oninput
                  ref=self.input_ref.clone() />
                <div class=send_button_class onclick=onclick>
                    <div class="icon send small"></div>
                </div>
            </div>
        }
    }
}
