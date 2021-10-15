use yew::prelude::*;
#[derive(Properties, Clone, PartialEq)]

pub struct Props {
    pub left: Option<Html>,
    pub center: Option<Html>,
    pub right: Option<Html>,
}

pub struct WindowManager {
    link: ComponentLink<Self>,
    props: Props,
    old_left: Option<Html>,
    old_right: Option<Html>,
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

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        WindowManager {
            props,
            link,
            old_left: None,
            old_right: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Switch(panel) => {
                match panel {
                    PanelSide::Left => self.old_left = None,
                    PanelSide::Right => self.old_right = None,
                }
                true
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props.left != self.props.left {
            std::mem::swap(&mut self.old_left, &mut self.props.left);
            self.props.left = props.left;
        }
        if props.right != self.props.right {
            std::mem::swap(&mut self.old_right, &mut self.props.right);
            self.props.right = props.right;
        }
        if props.center != self.props.center {
            self.props.center = props.center
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
                    { optional_side(self.props.left.clone(), self.old_left.clone(), PanelSide::Left, left_switch_cb) }
                    <main class="main-window">
                        {main}
                    </main>
                    { optional_side(self.props.right.clone(), self.old_right.clone(), PanelSide::Right, right_switch_cb) }
                </div>
            </>
        }
    }
}

fn optional_side(
    mut content: Option<Html>,
    mut old: Option<Html>,
    side: PanelSide,
    transition_cb: Callback<TransitionEvent>,
) -> Html {
    let panel_side_class = match side {
        PanelSide::Left => "panel-left",
        PanelSide::Right => "panel-right",
    };

    let do_switch = old.is_some();

    html!(
        <>
            // old-panel (the one that is visible most of the time and can also be called current panel)
            <section class=classes!("side-window", panel_side_class,
                content.is_none().then(|| "closed")) style="z-index: 2">
            {
                old.take().unwrap_or_else(|| content.take().unwrap_or(html!()))
            }
            </section>
            // new panel (the one that gets swiped over the old one)
            <section ontransitionend=transition_cb class=classes!("side-window",
                (!do_switch).then(|| "closed"), panel_side_class, "switch-window") style="z-index: 1">
            {
                do_switch.then(|| content.take().unwrap()).unwrap_or(html!())
            }
            </section>

        </>
    )
}
