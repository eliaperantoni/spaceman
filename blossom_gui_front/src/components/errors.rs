use yew::*;

pub struct Errors {
}

#[derive(PartialEq, Properties)]
pub struct ErrorsProps {
    pub errors: Vec<String>,
    pub dismiss_error: Callback<usize>,
}

impl Component for Errors {
    type Message = ();
    type Properties = ErrorsProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="errors">
                { ctx.props().errors.iter().enumerate().map(|(idx, error)| {
                    html!{
                        <div class="error" onclick={
                            let cb = ctx.props().dismiss_error.clone();
                            move |_| {
                                cb.emit(idx);
                            }
                        }>{ error.clone() }</div>
                    }
                }).collect::<Html>() }
            </div>
        }
    }
}
