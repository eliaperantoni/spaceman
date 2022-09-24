use yew::prelude::*;

#[function_component(Ui)]
fn ui() -> Html {
    html! {
        <h1>{ "Hello!" }</h1>
    }
}

fn main() {
    yew::start_app::<Ui>();
}
