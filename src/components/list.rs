use log::*;
use std::rc::Rc;
use yew::{html, Callback, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone)]
pub struct Props<T: Clone + PartialEq> {
    pub class: String,
    pub list: Irc<Vec<T>>,
    pub list_range: Irc<(usize, usize)>,
    pub list_len: Irc<usize>,
    pub selected_id: Irc<Option<u32>>,
    pub fetch_callback: Callback<(usize, usize)>,
    pub render_element: Rc<dyn Fn(T) -> Html>,
}

impl<T: Clone + PartialEq> PartialEq for Props<T> {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
            && self.list == other.list
            && self.list_range == other.list_range
            && self.list_len == other.list_len
            && self.selected_id == other.selected_id
            && self.fetch_callback == other.fetch_callback
            && Rc::ptr_eq(&self.render_element, &other.render_element)
    }
}

pub struct List<T: Clone + PartialEq + 'static> {
    props: Props<T>,
    link: ComponentLink<Self>,
    list_ref: NodeRef,
    scroll_bottom_next: bool,
    scroll: (i32, i32),
    loading: bool,
}

impl<T: Clone + PartialEq + 'static> List<T> {
    fn list_div(&self) -> web_sys::Element {
        self.list_ref.cast::<web_sys::Element>().unwrap()
    }

    fn scroll_to_bottom(&mut self) {
        let el = self.list_div();
        let mut opts = web_sys::ScrollToOptions::new();
        let scroll_height = el.scroll_height();
        opts.top(scroll_height as f64);
        el.scroll_to_with_scroll_to_options(&opts);
        self.scroll_bottom_next = false;
    }

    fn scroll_to_last(&self) {
        info!("scroll to last");
        let el = self.list_div();

        let new_scroll = el.scroll_height() - el.client_height();
        el.set_scroll_top(self.scroll.0 + (new_scroll - self.scroll.1));
    }
}

pub enum Msg {
    OnScroll(web_sys::Event),
}

impl<T: Clone + PartialEq + 'static> Component for List<T> {
    type Message = Msg;
    type Properties = Props<T>;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        List {
            props,
            link,
            list_ref: NodeRef::default(),
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
                    list_range,
                    list_len,
                    ..
                } = &self.props;
                let list_len = **list_len;

                info!("-----");
                if self.loading {
                    return false;
                }

                let range = if **list_range == (0, 0) {
                    let init = (list_len.saturating_sub(batch_size), list_len);
                    info!("load initial");
                    // load initial size
                    Some(init)
                } else if list_len < max_window_size {
                    info!("loading all {} {}", max_window_size, list_len);
                    // load all
                    Some((0, max_window_size))
                } else {
                    info!("window ({}, {})", list_range.0, list_range.1);

                    // We have only a subview, not showing everything

                    let el = self.list_div();

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

                        let current_window_size = list_range.1 - list_range.0;

                        let start_index = list_range.0.saturating_sub(batch_size);

                        let stop_index = if current_window_size > max_window_size {
                            // remove one batch from the bottom
                            list_range
                                .1
                                .saturating_sub(batch_size)
                                .min(start_index + batch_size)
                        } else {
                            list_range.1
                        };

                        Some((start_index, stop_index))
                    } else if is_end {
                        info!("Load more (bottom) {} ({:?})", from_bottom, *list_range);

                        let current_window_size = list_range.1 - list_range.0;

                        let stop_index = (list_range.1 + batch_size).min(list_len);

                        let start_index = if current_window_size > max_window_size {
                            // remove one batch from the top
                            (list_range.0 + batch_size).min(stop_index - batch_size)
                        } else {
                            list_range.0
                        };

                        Some((start_index, stop_index))
                    } else {
                        None
                    }
                };

                if let Some(range) = range {
                    if range.0 != list_range.0 || range.1 != list_range.1 {
                        self.loading = true;
                        self.props.fetch_callback.emit(range);
                    }
                }

                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let chat_id_changed = self.props.selected_id != props.selected_id;
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
          <div class=&self.props.class ref=self.list_ref.clone() onscroll=onscroll>
            { self.props.list.iter().map(|msg| (self.props.render_element)(msg.clone())).collect::<Html>() }
          </div>
        }
    }
}
