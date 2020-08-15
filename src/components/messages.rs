use log::*;
use shared::*;
use yew::{html, Callback, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};
use yewtil::NeqAssign;

use crate::components::message::Message;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub messages: Vec<ChatMessage>,
    pub messages_len: usize,
    pub messages_range: (usize, usize),
    pub selected_chat_id: Option<u32>,
    pub fetch_callback: Callback<(usize, usize)>,
}

pub struct Messages {
    props: Props,
    link: ComponentLink<Messages>,
    messages_ref: NodeRef,
    scroll_bottom_next: bool,
    scroll: (i32, i32),
}

impl Messages {
    fn messages_div(&self) -> web_sys::Element {
        self.messages_ref.cast::<web_sys::Element>().unwrap()
    }

    fn scroll_to_bottom(&mut self) {
        let el = self.messages_div();
        let mut opts = web_sys::ScrollToOptions::new();
        let scroll_height = el.scroll_height();
        opts.top(scroll_height as f64);
        el.scroll_to_with_scroll_to_options(&opts);
        self.scroll_bottom_next = false;
    }

    fn scroll_to_last(&self) {
        info!("scroll to last");
        let el = self.messages_div();

        let new_scroll = el.scroll_height() - el.client_height();
        el.set_scroll_top(self.scroll.0 + (new_scroll - self.scroll.1));
    }
}

pub enum Msg {
    OnScroll(web_sys::Event),
}

impl Component for Messages {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Messages {
            props,
            link,
            messages_ref: NodeRef::default(),
            scroll_bottom_next: true,
            scroll: (0, 0),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::OnScroll(_ev) => {
                let el = self.messages_div();

                let from_top = el.scroll_top();
                let from_bottom = el.scroll_height() - el.scroll_top();
                self.scroll = (el.scroll_top(), el.scroll_height() - el.client_height());

                // element.scrollHeight - element.scrollTop === element.clientHeight
                let is_end = el.scroll_height() - el.scroll_top() == el.client_height();

                let Props {
                    messages_len,
                    messages_range,
                    ..
                } = self.props;

                if from_top < 20 && messages_range.0 > 0 {
                    info!("Load more (top) {}", from_top);

                    let start_index = if messages_range.0 < 20 {
                        0
                    } else {
                        messages_range.0.saturating_sub(20)
                    };
                    let len = (messages_range.1.saturating_sub(start_index)).max(50);
                    let stop_index = (start_index + len).min(messages_len);

                    self.props.fetch_callback.emit((start_index, stop_index));
                } else if is_end {
                    info!(
                        "Load more (bottom) {} ({:?}, {})",
                        from_bottom, messages_range, messages_len
                    );

                    // let stop_index = if messages_len.saturating_sub(messages_range.1) < 20 {
                    //     messages_len
                    // } else {
                    //     messages_range.1 + 20
                    // };
                    // let len = (stop_index.saturating_sub(messages_range.0)).max(50);
                    // let start_index = stop_index.saturating_sub(len);

                    // self.send_app_message(Msg::WsRequest(Request::LoadMessageList {
                    //     start_index,
                    //     stop_index,
                    // }));
                }

                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let chat_id_changed = self.props.selected_chat_id != props.selected_chat_id;
        let new_props = self.props.neq_assign(props);
        if new_props {
            self.scroll_bottom_next = chat_id_changed;
        }

        new_props
    }

    fn rendered(&mut self, first_render: bool) {
        if self.scroll_bottom_next || first_render {
            self.scroll_to_bottom();
        } else {
            self.scroll_to_last();
        }
    }

    fn view(&self) -> Html {
        let onscroll = self.link.callback(Msg::OnScroll);

        html! {
            <div class="message-list" ref=self.messages_ref.clone() onscroll=onscroll>
            { self.props.messages.iter().map(|msg| {
                html! {
                    <Message message=msg />
                }
            }).collect::<Html>() }
            </div>
        }
    }
}
