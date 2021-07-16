use std::collections::HashMap;

use log::*;
use shared::{ChatState, SharedAccountState};
use validator::{Validate, ValidationError};
use yew::{html, Callback, Component, ComponentLink, Html, MouseEvent, Properties, ShouldRender};
use yew_form::{Field, Form};
use yewtil::{ptr::Irc, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub cancel_callback: Callback<()>,
}

pub struct Modal {
    props: Props,
    link: ComponentLink<Self>,
    form: Form<Login>,
}

#[derive(yew_form_derive::Model, Validate, PartialEq, Clone, Debug)]
struct Login {
    #[validate(email(message = "Must be a valid email"))]
    email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    password: String,
}

impl Default for Login {
    fn default() -> Self {
        Login {
            email: String::new(),
            password: String::new(),
        }
    }
}

#[derive(Debug)]
pub enum Msg {
    FormUpdate,
    Submit,
}

impl Component for Modal {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Modal {
            props,
            link,
            form: Form::new(Login::default()),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FormUpdate => true,
            Msg::Submit => {
                let valid = self.form.validate();
                info!("submitted  (valid: {})", valid);
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let cb = self.props.cancel_callback.clone();
        let submit = self.link.callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::Submit
        });
        let cancel: Callback<_> = (move |_| cb.emit(())).into();

        html! {
          <div class="modal-window">
            <div class="account-create">
              <h1>{"Login"}</h1>
              <form>
                <div class="form-group">
                  <label for="email">{"Email"}</label>
                  <Field<Login>
                    form=&self.form
                    field_name="email"
                    oninput=self.link.callback(|_| Msg::FormUpdate) />
                  <div class="invalid-feedback">
                    {&self.form.field_message("email")}
                  </div>

                  <label for="password">{"Password"}</label>
                  <Field<Login>
                    form=&self.form
                    field_name="password"
                    input_type="password"
                    oninput=self.link.callback(|_| Msg::FormUpdate) />
                  <div class="invalid-feedback">
                    {&self.form.field_message("password")}
                  </div>
                </div>

                <div class="form-group">
                  <button
                    type="button"
                    class="modal-close icon close small"
                    onclick=cancel>
                  </button>
                  <button
                    class="submit-button"
                    type="button"
                    onclick=submit>
                    {"Login"}
                  </button>
                </div>
              </form>
            </div>
          </div>
        }
    }
}
