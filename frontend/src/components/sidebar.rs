use std::collections::HashMap;

use shared::SharedAccountState;
use yew::{html, Callback, Component, ComponentLink, Html, Properties, ShouldRender};
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub accounts: Irc<HashMap<String, SharedAccountState>>,
    pub selected_account: Irc<Option<String>>,
    pub create_account_callback: Callback<()>,
    pub select_account_callback: Callback<String>,
}

pub struct Sidebar {
    props: Props,
    link: ComponentLink<Self>,
}

impl Component for Sidebar {
    type Message = ();
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Sidebar { props, link }
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
        html! {
            <div class="sidebar">
                <div class="account-list">
                    { self.props.accounts.iter().map(|(_, acc)| {
                        let account = acc.email.clone();
                        let cb = self.props.select_account_callback.clone();
                        let onclick: Callback<_> = (move |_| cb.emit(account.to_string())).into();
                        html! {
                            <div class="account" onclick=onclick>
                                <div class="letter-icon" >
                                    {acc.email.chars().next().unwrap()}
                                </div>
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
