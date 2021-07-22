use log::*;

use validator::Validate;
use yew::{html, Callback, Component, ComponentLink, Html, MouseEvent, Properties, ShouldRender};
use yew_form::{Field, Form};
use yewtil::NeqAssign;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub cancel_callback: Callback<()>,
}

#[derive(PartialEq)]
enum CreateType {
    Login,
    Import,
}

impl CreateType {
    fn next(&mut self) {
        match &self {
            CreateType::Login => *self = CreateType::Import,
            CreateType::Import => *self = CreateType::Login,
        }
    }
}

pub struct Modal {
    props: Props,
    link: ComponentLink<Self>,
    login_form: Form<Login>,
    import_form: Form<ImportForm>,
    create_type: CreateType,
}

#[derive(yew_form_derive::Model, Validate, PartialEq, Clone, Debug)]
struct Login {
    #[validate(email(message = "Must be a valid email"))]
    email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    password: String,
}

#[derive(yew_form_derive::Model, Validate, PartialEq, Clone, Debug, Default)]
struct ImportForm {
    #[validate(email(message = "Must be a valid email"))]
    email: String,
    path: String,
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
    Switch,
}

impl Component for Modal {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Modal {
            props,
            link,
            login_form: Form::new(Login::default()),
            import_form: Form::new(ImportForm::default()),
            create_type: CreateType::Login,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FormUpdate => true,
            Msg::Submit => {
                match self.create_type {
                    CreateType::Login => {
                        let valid = self.login_form.validate();
                        info!("submitted  (valid: {})", valid);
                    }
                    CreateType::Import => {
                        let valid = self.import_form.validate();
                        info!("submitted  (valid: {})", valid);
                        info!("path-value: {}", self.import_form.field_value("path"))
                    }
                }
                true
            }
            Msg::Switch => {
                self.create_type.next();
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
        let switch = self.link.callback(|_e: MouseEvent| Msg::Switch);
        let cancel: Callback<_> = (move |_| cb.emit(())).into();

        let input_form = move || -> Html {
            if self.create_type == CreateType::Login {
                html! {
                    <form>
                        <div class="form-group">
                        <label for="email">{"Email"}</label>
                        <Field<Login>
                            form=&self.login_form
                            field_name="email"
                            oninput=self.link.callback(|_| Msg::FormUpdate) />
                        <div class="invalid-feedback">
                            {&self.login_form.field_message("email")}
                        </div>

                        <label for="password">{"Password"}</label>
                        <Field<Login>
                            form=&self.login_form
                            field_name="password"
                            input_type="password"
                            oninput=self.link.callback(|_| Msg::FormUpdate) />
                        <div class="invalid-feedback">
                            {&self.login_form.field_message("password")}
                        </div>
                        </div>

                        <div class="form-group">
                            <button
                                class="submit-button"
                                type="button"
                                onclick=submit>
                                {"Login"}
                            </button>
                        </div>
                    </form>
                }
            } else {
                html! {
                    <form>
                        <div class="form-group">
                            <label for="email">{"Email"}</label>
                            <Field<ImportForm>
                                form=&self.import_form
                                field_name="email"
                                oninput=self.link.callback(|_| Msg::FormUpdate) />
                            <div class="invalid-feedback">
                                {&self.import_form.field_message("email")}
                            </div>

                            <label for="path">{"Pfad"}</label>
                            <Field<ImportForm>
                                form=&self.import_form
                                field_name="path"
                                input_type="file"
                                oninput=self.link.callback(|_| Msg::FormUpdate) />
                        </div>

                        <div class="form-group">
                            <button
                                class="submit-button"
                                type="button"
                                onclick=submit>
                                {"Import"}
                            </button>
                        </div>
                    </form>
                }
            }
        };

        html! {
            <div class="modal-window">
                <div class="account-create">
                // select login-type
                <div class="select-type">
                    <h1 onclick=switch.clone()>{"Login"}</h1>
                    <h1 class="spacer">{"|"}</h1>
                    <h1 onclick=switch>{"Import"}</h1>
                </div>

                {input_form()}
                <button
                        type="button"
                        class="modal-close icon close small"
                        onclick=cancel>
                    </button>
                </div>
            </div>
        }
    }
}
