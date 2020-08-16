use log::*;
use shared::*;
use yew::{html, Callback, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

use crate::components::message::Message;

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
    messages_ref: NodeRef,
    scroll_bottom_next: bool,
    scroll: (i32, i32),
    loading: bool,
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
            loading: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        let batch_size = 30;
        let max_window_size = 3 * batch_size;

        // At which pixel distance we start loading
        let max_top_pixels = 20;

        match msg {
            Msg::OnScroll(_ev) => {
                let Props {
                    messages_range,
                    messages_len,
                    ..
                } = &self.props;
                let messages_len = **messages_len;

                info!("-----");
                if self.loading {
                    return false;
                }

                let range = if **messages_range == (0, 0) {
                    let init = (messages_len.saturating_sub(batch_size), messages_len);
                    info!("load initial");
                    // load initial size
                    Some(init)
                } else if messages_len < max_window_size {
                    info!("loading all {} {}", max_window_size, messages_len);
                    // load all
                    Some((0, max_window_size))
                } else {
                    info!("window ({}, {})", messages_range.0, messages_range.1);

                    // We have only a subview, not showing everything

                    let el = self.messages_div();

                    // distance to currently loaded top end
                    let from_top = el.scroll_top();
                    // distance to currently loaded bottom end
                    let from_bottom = el.scroll_height() - el.scroll_top();
                    self.scroll = (el.scroll_top(), el.scroll_height() - el.client_height());

                    // element.scrollHeight - element.scrollTop === element.clientHeight
                    info!(
                        "{}, {}, {}",
                        el.scroll_height() - el.scroll_top(),
                        el.client_height(),
                        max_top_pixels
                    );
                    let is_end =
                        el.scroll_height() - el.scroll_top() <= el.client_height() + max_top_pixels;

                    if from_top < max_top_pixels {
                        // need to move to window upwards
                        info!("Load more (top) {}", from_top);

                        let current_window_size = messages_range.1 - messages_range.0;

                        let start_index = messages_range.0.saturating_sub(batch_size);

                        let stop_index = if current_window_size > max_window_size {
                            // remove one batch from the bottom
                            messages_range
                                .1
                                .saturating_sub(batch_size)
                                .min(start_index + batch_size)
                        } else {
                            messages_range.1
                        };

                        Some((start_index, stop_index))
                    } else if is_end {
                        info!("Load more (bottom) {} ({:?})", from_bottom, *messages_range);

                        let current_window_size = messages_range.1 - messages_range.0;

                        let stop_index = (messages_range.1 + batch_size).min(messages_len);

                        let start_index = if current_window_size > max_window_size {
                            // remove one batch from the top
                            (messages_range.0 + batch_size).min(stop_index - batch_size)
                        } else {
                            messages_range.0
                        };

                        Some((start_index, stop_index))
                    } else {
                        None
                    }
                };

                if let Some(range) = range {
                    if range.0 != messages_range.0 || range.1 != messages_range.1 {
                        self.loading = true;
                        self.props.fetch_callback.emit(range);
                    }
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
            self.loading = false;
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
