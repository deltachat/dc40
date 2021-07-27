use gloo::file::{callbacks::FileReader, File};
use log::*;
use validator::Validate;
use web_sys::HtmlInputElement;
use yew::{
    html, Callback, Component, ComponentLink, Html, MouseEvent, NodeRef, Properties, ShouldRender,
};
use yew_form::{Field, Form};
use yewtil::NeqAssign;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub submit_callback: Callback<(String, String)>,
    pub cancel_callback: Callback<()>,
    pub import_callback: Callback<(String, Vec<u8>)>,
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
    reader: Option<FileReader>,
}

#[derive(yew_form_derive::Model, Validate, PartialEq, Clone, Debug)]
pub struct Login {
    #[validate(email(message = "Must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
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
    Login,
    Import(File),
    Switch,
    LoadedBackup(Vec<u8>),
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
            // this field is only used to not drop the reader
            reader: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FormUpdate => true,
            Msg::Switch => {
                self.create_type.next();
                true
            }
            Msg::Login => {
                let valid = self.login_form.validate();
                info!("submitted  (valid: {})", valid);
                self.props.submit_callback.emit((
                    self.form.field_value("email"),
                    self.form.field_value("password"),
                ));
                true
            }
            Msg::Import(file) => {
                let valid = self.import_form.validate();
                info!("form_valid:{}", valid);
                info!("loading file: {:?}", file.name());
                let link = self.link.clone();
                let task = gloo::file::callbacks::read_as_bytes(&file, move |res| {
                    link.send_message(Msg::LoadedBackup(res.expect("failed to read file")))
                });
                self.reader = Some(task);
                true
            }
            Msg::LoadedBackup(data) => {
                info!("transfering backup");
                self.create_type.next();
                self.props
                    .import_callback
                    .emit((self.import_form.field_value("email"), data));
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let cb = self.props.cancel_callback.clone();
        let login = self.link.callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::Login
        });

        let node_ref = NodeRef::default();
        let node_ref_clone = node_ref.clone();
        let import = self.link.callback(move |e: MouseEvent| {
            e.prevent_default();
            let input = node_ref_clone.cast::<HtmlInputElement>().unwrap();
            let file = input.files().unwrap().get(0).unwrap();
            Msg::Import(file.into())
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
                                onclick=login>
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
                            <input
                                type="file"
                                ref={node_ref}
                                oninput=self.link.callback(|_| Msg::FormUpdate) />
                        </div>

                        <div class="form-group">
                            <button
                                class="submit-button"
                                type="button"
                                onclick=import>
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
