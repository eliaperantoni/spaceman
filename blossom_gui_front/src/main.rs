use web_sys::{console::log_1, HtmlElement};
use yew::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use std::cell::RefCell;

mod call;
mod commands;
mod glue;

#[function_component]
fn Sidebar() -> Html {
    html! {
        <div style="background: #E84855;"></div>
    }
}

#[function_component]
fn Main() -> Html {
    html! {
        <Pane>
            <div style="background: #1B998B"></div>
            <div style="background: #FFFD82"></div>
        </Pane>
    }
}

#[derive(PartialEq, Properties)]
struct PaneProps {
    children: Children,
}

#[derive(Debug)]
struct DragState {
    initial_pane_width: i32,
    initial_lhs_width: i32,
    initial_x: i32,
    onmousemove: Closure<dyn Fn(Event)>,
    onmouseup: Closure<dyn Fn(Event)>,
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

    let pane_ref = use_node_ref();
    let lhs_ref = use_node_ref();

    let drag_state = use_memo::<RefCell<Option<DragState>>, _, _>(
        |_| RefCell::new(None),
        ()
    );

    let onmousedown = Callback::from({
        let pane_ref = pane_ref.clone();
        let lhs_ref = lhs_ref.clone();

        let left_fraction = left_fraction.clone();
        move |ev: MouseEvent| {
            ev.prevent_default();

            match (pane_ref.cast::<HtmlElement>(), lhs_ref.cast::<HtmlElement>()) {
                (Some(pane), Some(lhs)) => {
                    let document = web_sys::window().expect("window to be present")
                        .document().expect("document to be present")
                        .document_element().expect("document to have at least an html element");

                    let onmousemove = Closure::<dyn Fn(Event)>::wrap({
                        let drag_state = drag_state.clone();
                        let left_fraction = left_fraction.clone();
                        Box::new(move |ev: Event| {
                            if let (Some(ev), Some(drag_state)) = (ev.dyn_ref::<MouseEvent>(), RefCell::borrow(&drag_state).as_ref()) {
                                let delta_x = ev.client_x() - drag_state.initial_x;
                                let width = drag_state.initial_lhs_width + delta_x;

                                left_fraction.set((width as f64 + (RESIZER_WIDTH as f64) / 2.0) / drag_state.initial_pane_width as f64);
                            }
                        })
                    });

                    let onmouseup = Closure::<dyn Fn(Event)>::wrap({
                        let drag_state = drag_state.clone();
                        let document = document.clone();
                        Box::new(move |ev: Event| {
                            let mut drag_state = RefCell::borrow_mut(&drag_state);
                            if drag_state.is_some() {
                                document.remove_event_listener_with_callback("mousemove", drag_state.as_ref().unwrap().onmousemove.as_ref().unchecked_ref());
                                document.remove_event_listener_with_callback("mouseup", drag_state.as_ref().unwrap().onmouseup.as_ref().unchecked_ref());
                                *drag_state = None;
                            }
                        })
                    });

                    document.add_event_listener_with_callback("mousemove", onmousemove.as_ref().unchecked_ref()).expect("can add listener");
                    document.add_event_listener_with_callback("mouseup", onmouseup.as_ref().unchecked_ref()).expect("can add listener");

                    let mut drag_state = drag_state.borrow_mut();
                    drag_state.insert(DragState {
                        initial_pane_width: pane.client_width(),
                        initial_lhs_width: lhs.client_width(),
                        initial_x: ev.client_x(),
                        onmousemove,
                        onmouseup,
                    });
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
