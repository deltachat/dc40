use yew::prelude::*;
use yewtil::NeqAssign;
#[derive(Properties, Clone, PartialEq)]


pub struct Props {
    pub left: Option<Html>, 
    pub center: Option<Html>,
    pub right: Option<Html>
}

pub struct WindowManager {
    link: ComponentLink<Self>,
    props: Props
}

pub enum Msg {

}

impl Component for WindowManager {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        WindowManager { props, link }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let m = html!(<p>{"No content"}</p>);
        let main = self.props.center.clone().unwrap_or(m.clone());
        html! {
            <div class="window">
                { optional_side(self.props.left.clone()) }
                <main class="main-window">
                    {main}
                </main>
                {optional_side(self.props.right.clone())}
            </div>
        }
    }
}

fn optional_side(content: Option<Html>) -> Html {
    match content {
        Some(content) => html!(
            <section class="side-window">
                {content}
            </section>
        ),
        None => html!(
            <section class="side-window closed">
            </section>
        )
    }
}
