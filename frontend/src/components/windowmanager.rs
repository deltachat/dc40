use yew::prelude::*;
#[derive(Debug)]
pub enum ChangePanel {
    Left(LeftPanel),
    Center,
    Right,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub left: Option<Html>,
    pub center: Option<Html>,
    pub right: Option<Html>,
    pub left_type: LeftPanel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LeftPanel {
    Chats,
    NewChat,
}

impl Default for LeftPanel {
    fn default() -> Self {
        Self::NewChat
    }
}

pub struct WindowManager {
    link: ComponentLink<Self>,
    props: Props,
    left: Option<Html>,
    right: Option<Html>,
    new_left: Option<Html>,
    new_right: Option<Html>,
}

pub enum PanelSide {
    Left,
    Right,
}

pub enum Msg {
    Switch(PanelSide),
}

impl Component for WindowManager {
    type Message = Msg;
    type Properties = Props;
    fn create(mut props: Self::Properties, link: ComponentLink<Self>) -> Self {
        WindowManager {
            link,
            left: props.left.take(),
            right: props.right.take(),
            props,
            new_left: None,
            new_right: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Switch(panel) => {
                match panel {
                    PanelSide::Left => {
                        self.left = Some(self.new_left.take().expect("must have a new_left"));
                        self.new_left = None;
                    }
                    PanelSide::Right => {
                        self.right = Some(self.new_right.take().expect("must have a new_right"));
                        self.new_right = None
                    }
                }
                true
            }
        }
    }
    fn change(&mut self, mut props: Self::Properties) -> ShouldRender {
        // Different view so we do a transition-effect
        if props.left_type != self.props.left_type {
            self.props.left_type = props.left_type;

            if props.left != self.left {
                self.new_left = props.left;
            }
            if props.right != self.right {
                self.new_right = props.right;
            }
            if props.center != self.props.center {
                self.props.center = props.center
            }
        } else {
            self.left = props.left.take();
            self.right = props.right.take();
            self.props = props;

        }
        true
    }

    fn view(&self) -> Html {
        let m = html!(<p>{"No content"}</p>);
        let main = self.props.center.clone().unwrap_or(m.clone());

        let left_switch_cb = self.link.callback(|_| Msg::Switch(PanelSide::Left));
        let right_switch_cb = self.link.callback(|_| Msg::Switch(PanelSide::Left));

        html! {
            <>
                <div class="window">
                    { optional_side(self.left.clone(), self.new_left.clone(), PanelSide::Left, left_switch_cb) }
                    <main class="main-window">
                        {main}
                    </main>
                    { optional_side(self.right.clone(), self.new_right.clone(), PanelSide::Right, right_switch_cb) }
                </div>
            </>
        }
    }
}

fn optional_side(
    content: Option<Html>,
    slide: Option<Html>,
    side: PanelSide,
    transition_cb: Callback<TransitionEvent>,
) -> Html {
    let panel_side_class = match side {
        PanelSide::Left => "panel-left",
        PanelSide::Right => "panel-right",
    };

    let slide = match slide {
        Some(content) => html! {
            <section ontransitionend=transition_cb class=classes!("side-window", panel_side_class, "switch-window", "in") style="z-index: 2">
            { content }
            </section>
        },
        None => html!(
            <section class=classes!("side-window", panel_side_class, "switch-window") style="z-index: 2">
            </section>
        ),
    };

    let content = match content {
        Some(content) => html! {
            <section class=classes!("side-window", panel_side_class) style="z-index: 1">
            {
                content
            }
            </section>
        },
        None => html!(),
    };

    html!(
        <>
            { content }
            { slide }

        </>
    )
}
