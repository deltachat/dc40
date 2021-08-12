//! Context Menu

use std::{collections::HashMap, convert::TryInto, time::Duration};

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::HtmlElement;
use yew::{
    html,
    html::ChildrenRenderer,
    services::{timeout::TimeoutTask, TimeoutService},
    Callback, Component, ComponentLink, Html, MouseEvent, NodeRef, Properties, ShouldRender,
};
use yewtil::NeqAssign;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub children: ChildrenRenderer<Html>,
    pub actions: HashMap<String, Callback<()>>,
}

pub struct ContextMenu {
    props: Props,
    link: ComponentLink<Self>,
    show_menu: bool,
    x: i32,
    y: i32,
    onclick: Option<Closure<dyn FnMut(MouseEvent)>>,
    timeout: Option<TimeoutTask>,
    menu_ref: NodeRef,
}

pub enum Msg {
    OnContextMenu { x: i32, y: i32 },
    AddEventListeners,
    Hide,
}

impl Component for ContextMenu {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        ContextMenu {
            props,
            link,
            show_menu: false,
            x: 0,
            y: 0,
            onclick: None,
            timeout: None,
            menu_ref: NodeRef::default(),
        }
    }

    fn destroy(&mut self) {
        self.remove_event_listeners();
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::OnContextMenu { mut x, mut y } => {
                let window = yew::utils::window();
                let window_width: i32 =
                    window.inner_width().unwrap().as_f64().unwrap() as u32 as i32;
                let window_height: i32 =
                    window.inner_height().unwrap().as_f64().unwrap() as u32 as i32;

                let menu_ref = self.menu_ref.cast::<HtmlElement>().unwrap();
                let menu_width: i32 = menu_ref.offset_width();
                let menu_height: i32 = menu_ref.offset_height();

                if x + menu_width > window_width {
                    x -= x + menu_width - window_width;
                }

                if y + menu_height > window_height {
                    y -= y + menu_height - window_height;
                }

                let should_update = self.show_menu == false || self.x != x || self.y != y;
                self.show_menu = true;
                self.x = x;
                self.y = y;
                if should_update && self.onclick.is_none() {
                    let add_callback = self.link.callback(|_| Msg::AddEventListeners);
                    self.timeout =
                        Some(TimeoutService::spawn(Duration::from_secs(0), add_callback));
                }
                should_update
            }
            Msg::AddEventListeners => {
                self.add_event_listeners();
                false
            }
            Msg::Hide => {
                let should_update = self.show_menu == true;
                self.show_menu = false;
                if should_update {
                    self.remove_event_listeners();
                }
                should_update
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let oncontextmenu = self.link.callback(|e: MouseEvent| {
            e.prevent_default();
            let mut x = e.client_x();
            let mut y = e.client_y();
            if x < 0 {
                x = 0;
            }
            if y < 0 {
                y = 0;
            }

            Msg::OnContextMenu { x, y }
        });
        html! {
            <div oncontextmenu=oncontextmenu ref=self.menu_ref.clone() >
              { self.render_menu() }
              { self.props.children.clone() }
            </div>
        }
    }
}

impl ContextMenu {
    fn render_menu(&self) -> Html {
        if !self.show_menu {
            return html! {};
        }

        let style = format!("top: {}px; left: {}px", self.y, self.x);
        let items: Vec<_> = self
            .props
            .actions
            .iter()
            .map(|(name, action)| {
                let action = action.clone();
                let on_click: Callback<_> = (move |_| {
                    action.emit(());
                })
                .into();

                html! {
                    <div class="item" onclick=on_click>
                      {name}
                    </div>
                }
            })
            .collect();
        html! {
            <div id="context-menu" style=style>
                {items}
            </div>
        }
    }

    fn add_event_listeners(&mut self) {
        let onclick = self.link.callback(|_e: MouseEvent| Msg::Hide);
        let onclick =
            Closure::wrap(Box::new(move |ev: MouseEvent| onclick.emit(ev.into()))
                as Box<dyn FnMut(MouseEvent)>);

        let document = yew::utils::document();
        let add_handler = |event: &str, onclick: &Closure<_>| {
            document
                .add_event_listener_with_callback(event, onclick.as_ref().unchecked_ref())
                .unwrap();
        };
        add_handler("click", &onclick);
        add_handler("contextmenu", &onclick);
        add_handler("resize", &onclick);
        add_handler("scroll", &onclick);

        self.onclick = Some(onclick);
    }

    fn remove_event_listeners(&mut self) {
        if let Some(onclick) = self.onclick.take() {
            let document = yew::utils::document();

            let remove_handler = |event: &str, onclick: &Closure<_>| {
                document
                    .remove_event_listener_with_callback(event, onclick.as_ref().unchecked_ref())
                    .unwrap();
            };
            remove_handler("click", &onclick);
            remove_handler("contextmenu", &onclick);
            remove_handler("resize", &onclick);
            remove_handler("scroll", &onclick);
        }
    }
}
