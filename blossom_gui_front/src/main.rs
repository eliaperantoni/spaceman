use web_sys::{console::log_1, HtmlElement};
use yew::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

mod call;
mod commands;
mod glue;

#[function_component]
fn Sidebar() -> Html {
    html! {
        <div style="background: red;"></div>
    }
}

#[function_component]
fn Main() -> Html {
    html! {
        <div style="background: blue;"></div>
    }
}

#[derive(PartialEq, Properties)]
struct PaneProps {
    children: Children,
}

struct DragState {
    initial_pane_width: i32,
    initial_lhs_width: i32,
    initial_x: i32,
}

#[function_component]
fn Pane(props: &PaneProps) -> Html {
    const RESIZER_WIDTH: i32 = 8;

    let [lhs, rhs]: [_; 2] = props
        .children
        .iter()
        .collect::<Vec<_>>()
        .try_into()
        .expect("pane to have two children");

    let left_fraction = use_state(|| 0.2);

    // let handle = use_node_ref();
    // use_effect_with_deps(
    //     {
    //         let handle = handle.clone();

    //         move |_| {
    //             log::info!("setting listener");

    //             let mut listener = None;

    //             if let Some(handle) = handle.cast::<HtmlElement>() {
    //                 let on_mousedown = Closure::<dyn Fn(Event)>::wrap(
    //                     Box::new(move |ev: Event| {
    //                         log::info!("hallo");
    //                     })
    //                 );

    //                 handle.add_event_listener_with_callback(
    //                     "mousedown",
    //                     on_mousedown.as_ref().unchecked_ref(),
    //                 ).unwrap();

    //                 listener = Some(on_mousedown);
    //             }

    //             move || {
    //                 log::info!("unsetting listener");
    //                 drop(listener);
    //             }
    //         }
    //     },
    //     handle.clone()
    // );

    let onmouseup = Callback::from() {

    };

    let pane_ref = use_node_ref();
    let lhs_ref = use_node_ref();

    let drag_state = use_state::<Option<DragState>, _>(|| None);

    let onmousedown = Callback::from({
        let pane_ref = pane_ref.clone();
        let lhs_ref = lhs_ref.clone();

        move |ev: MouseEvent| {
            ev.prevent_default();

            match (pane_ref.cast::<HtmlElement>(), lhs_ref.cast::<HtmlElement>()) {
                (Some(pane), Some(lhs)) => {
                    drag_state.set(Some(DragState {
                        initial_pane_width: pane.client_width(),
                        initial_lhs_width: lhs.client_width(),
                        initial_x: ev.client_x(),
                    }));
                },
                _ => ()
            };
        }
    });

    html! {
        <div class="pane" ref={ pane_ref }>
            <div class="lhs" ref={ lhs_ref }
                style={
                    format!("width: calc({}% - {}px);",
                    *left_fraction * 100.0,
                    RESIZER_WIDTH / 2)
                }>{ lhs }</div>
            <div class="resizer" style={ format!("width: {}px;", RESIZER_WIDTH) }>
                <div class="line"/>
                <div class="handle" { onmousedown }/>
                <div class="line"/>
            </div>
            <div class="rhs">{ rhs }</div>
        </div>
    }
}

enum UiMsg {}

struct Ui {}

impl Component for Ui {
    type Properties = ();
    type Message = UiMsg;

    fn create(ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="ui">
                <Pane>
                    <Sidebar/>
                    <Main/>
                </Pane>
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<Ui>::new().render();
}
