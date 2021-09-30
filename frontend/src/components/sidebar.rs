use std::collections::HashMap;

use shared::SharedAccountState;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub accounts: Irc<HashMap<u32, SharedAccountState>>,
    pub selected_account: Irc<Option<u32>>,
    pub create_account_callback: Callback<()>,
    pub select_account_callback: Callback<u32>,
}

pub struct Sidebar {
    props: Props,
}

impl Component for Sidebar {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Sidebar { props }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let cb = self.props.create_account_callback.clone();
        let onclick: Callback<_> = (move |_| cb.emit(())).into();
        let selected_account = self.props.selected_account.unwrap_or_default();

        html! {
            <div class="sidebar">
                <div class="account-list">
                    { self.props.accounts.iter().map(|(id, acc)| {
                        let cb = self.props.select_account_callback.clone();
                        let id = *id;
                        let onclick: Callback<_> = (move |_| cb.emit(id)).into();
                        let mut cls = "account".to_string();
                        if id == selected_account {
                            cls += " active";
                        }
                        let image = if let Some(ref profile_image) = acc.profile_image {
                            let src = format!("asset://{}", profile_image.to_string_lossy());

                            html! {
                                <img
                                 class="image-icon"
                                 src=src
                                 alt="chat avatar" />
                            }
                        } else {
                            html! {
                              <div class="letter-icon">
                                {acc.email.chars().next().unwrap_or_default()}
                              </div>
                            }
                        };

                        html! {
                            <div class=cls onclick=onclick>
                                {image}
                            </div>
                        }
                    }).collect::<Html>() }
                    <a class="account add" onclick=onclick>
                        <div class="icon add medium"></div>
                    </a>
                </div>
            </div>
        }
    }
}
