use yew::prelude::*;
use yewtil::NeqAssign;
#[derive(Properties, Clone, PartialEq)]


pub struct Props {
    pub left: Option<Html>, 
    pub center: Option<Html>,
    pub right: Option<Html>
}

pub struct FileManager {
    link: ComponentLink<Self>,
    props: Props
}

pub enum Msg {

}

impl Component for FileManager {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        FileManager { props, link }
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
                <div class="main-window">
                    {main}
                </div>
                {optional_side(self.props.right.clone())}
            </div>
        }
    }
}

fn optional_side(content: Option<Html>) -> Html {
    match content {
        Some(content) => html!(
            <div class="side-window">
                {content}
            </div>
        ),
        None => html!(
            <div class="side-window closed">
            </div>
        )
    }
}


// we might want to use this later 
/* 
#[derive(Debug)]
pub enum LeftOptions {
    ChatList,
    Idk,
    None,
}

impl LeftOptions {
    fn next(&mut self){
        match self {
            LeftOptions::ChatList => *self = LeftOptions::Idk,
            LeftOptions::Idk => *self = LeftOptions::None,
            LeftOptions::None => *self = LeftOptions::ChatList,
        }
    }
}

#[derive(Debug)]
pub enum CenterOptions {
    Chat,
    None,
}
#[derive(Debug)]
pub enum RightOptions {
    Files,
    None,
}

impl RightOptions {
    fn next(&mut self){
        match self {
            RightOptions::Files => *self = RightOptions::None,
            RightOptions::None => *self = RightOptions::Files,
        }
    }
}

pub struct Window {
    pub left: LeftOptions,
    pub right: RightOptions,
    pub center: CenterOptions,
} */