use log::*;

use wasm_bindgen::prelude::*;
use yew::{html, Callback, Component, ComponentLink, Html, MouseEvent, Properties, ShouldRender};
use yew_form::{Field, Form};
use yewtil::{future::LinkFuture, NeqAssign};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub submit_callback: Callback<(String, String)>,
    pub cancel_callback: Callback<()>,
    pub import_callback: Callback<u32>,
}

pub struct Modal {
    props: Props,
    link: ComponentLink<Self>,
    form: Form<Login>,
}

#[derive(yew_form_derive::Model, Validate, PartialEq, Clone, Debug)]
pub struct Login {
    #[validate(email(message = "Must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
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
    Import(u32),
    RequestImport,
}

#[wasm_bindgen(module = "/src/js/tauri_wrapper.js")]
extern "C" {
    async fn invoke_backup_import() -> JsValue;
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
                self.props.submit_callback.emit((
                    self.form.field_value("email"),
                    self.form.field_value("password"),
                ));
                true
            }
            Msg::Import(id) => {
                info!("requesting account-data for account with id: {}", id);
                self.props.import_callback.emit(id);
                self.props.cancel_callback.emit(());
                true
            }
            Msg::RequestImport => {
                self.link.send_future(async {
                    let t = unsafe { invoke_backup_import().await };
                    Msg::Import(t.as_f64().unwrap() as u32)
                });
                false
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

        let import = self.link.callback(|_| Msg::RequestImport);

        html! {
          <div class="modal-window">
            <div class="account-create">
              <h1>{"Login"}</h1>
              <form>
                <div class="form-group">
                  <label for="email">{"Email"}</label>
                  <Field<Login>
                    form=self.form.clone()
                    field_name="email"
                    oninput=self.link.callback(|_| Msg::FormUpdate) />
                  <div class="invalid-feedback">
                    {&self.form.field_message("email")}
                  </div>

                  <label for="password">{"Password"}</label>
                  <Field<Login>
                    form=self.form.clone()
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
              <p class="or-spacer">{"--- or ---"}</p>
              <button class="submit-button" onclick=import id="acc_import_button">{"Import Backup"}</button>
              </div>
          </div>
        }
    }
}
