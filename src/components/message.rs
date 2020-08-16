use shared::*;
use yew::{html, virtual_dom::VList, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::NeqAssign;

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

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.message.neq_assign(props.message)
    }

    fn view(&self) -> Html {
        match &self.message {
            ChatMessage::Message {
                from_color,
                from_first_name,
                from_profile_image,
                viewtype,
                text,
                file,
                file_height,
                is_info,
                id,
                timestamp,
                ..
            } => {
                let image_style = format!("background-color: #{}", from_color);
                let image = if let Some(ref profile_image) = from_profile_image {
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
                        {from_first_name.chars().next().unwrap()}
                        </div>
                    }
                };

                let file = match viewtype {
                    Viewtype::Image | Viewtype::Gif => {
                        if let Some(ref file) = file {
                            let file_height = (*file_height).max(200);
                            html! {
                                <div class="message-image">
                                    <img
                                    src={format!("dc://{}", file.display())}
                                alt="image"
                                    height={(file_height).min(400)}
                                width="auto" />
                                    </div>
                            }
                        } else {
                            html! {}
                        }
                    }
                    _ => html! {},
                };

                let text = text.as_ref().map(process_text).unwrap_or_default();
                let content = if *is_info {
                    html! {
                        <div class="message-info">{text}</div>
                    }
                } else {
                    html! {
                        <div class="message-text">
                            <div class="message-icon">{image}</div>
                            <div class="message-body">
                            <div class="message-header">
                            <div class="message-sender">{&from_first_name}</div>
                            <div class="message-timestamp">
                        {timestamp.format("%R")}
                        </div>
                            </div>
                        { file }
                        <div class="message-inner-text">
                        {text}
                        </div>
                            </div>
                            </div>
                    }
                };

                html! {
                    <div class="message" key=*id>
                    { content }
                    </div>
                }
            }
            ChatMessage::DayMarker(time) => {
                html! {
                    <div class="day-marker" key=time.timestamp()>
                        {time.format("%A, %B %-d")}
                    </div>
                }
            }
        }
    }
}

fn process_text(source: impl AsRef<str>) -> Html {
    let link_finder = linkify::LinkFinder::new();
    link_finder
        .spans(source.as_ref())
        .fold(VList::new(), |mut acc, span| {
            match span.kind() {
                Some(linkify::LinkKind::Url) => {
                    acc.add_child(html! {
                        <a target="_blank" href=span.as_str()>{span.as_str()}</a>
                    });
                }
                Some(linkify::LinkKind::Email) => {
                    acc.add_child(html! {
                        <a target="_blank" href=format!("mailto:{}", span.as_str())>{span.as_str()}</a>
                    });
                }
                None => acc.add_child(span.as_str().into()),
                _ => {}
            }

            acc
        })
        .into()
}
