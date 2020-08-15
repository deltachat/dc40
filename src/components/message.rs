use log::*;
use shared::*;
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub message: ChatMessage,
}

pub struct Message {
    message: ChatMessage,
    link: ComponentLink<Message>,
}

impl Component for Message {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Message {
            message: props.message,
            link,
        }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let msg = &self.message;
        let image_style = format!("background-color: #{}", msg.from_color);
        let image = if let Some(ref profile_image) = msg.from_profile_image {
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
                   {msg.from_first_name.chars().next().unwrap()}
                </div>
            }
        };

        let file = match msg.viewtype {
            Viewtype::Image | Viewtype::Gif => {
                if let Some(ref file) = msg.file {
                    info!("{}, {}", msg.file_height, msg.file_width);
                    html! {
                        <div class="message-image">
                          <img
                            src={format!("dc://{}", file.display())}
                            alt="image"
                            height=300
                            width="auto" />
                        </div>
                    }
                } else {
                    html! {}
                }
            }
            _ => html! {},
        };

        let content = if msg.is_info {
            html! {
                <div class="message-info">{msg.text.as_ref().cloned().unwrap_or_default()}</div>
            }
        } else {
            html! {
                <div class="message-text">
                    <div class="message-icon">{image}</div>
                    <div class="message-body">
                    <div class="message-header">
                    <div class="message-sender">{&msg.from_first_name}</div>
                    <div class="message-timestamp">
                {msg.timestamp.format("%R")}
                </div>
                    </div>
                { file }
                <div class="message-inner-text">
                {msg.text.as_ref().cloned().unwrap_or_default()}
                </div>
                    </div>
                    </div>
            }
        };

        html! {
            <div class="message" key=msg.id>
                { content }
            </div>
        }
    }
}
