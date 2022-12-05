use web_sys::{console::log_1, HtmlElement};
use yew::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use std::cell::RefCell;

mod call;
mod commands;
mod glue;
mod components;

use components::pane::Pane;
use components::button::Button;

#[function_component]
fn Sidebar() -> Html {
    html! {
        <div class="sidebar">
            <Button
                text="Settings"
                icon="img/cog.svg"
                style="align-self: stretch"/>
        </div>
    }
}

#[function_component]
fn Main() -> Html {
    html! {
        <Pane initial_left={ 0.5 }>
            <div></div>
            <div></div>
        </Pane>
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
                <Pane initial_left={ 0.2 } style_lhs="min-width: 150px">
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
