use yew::prelude::*;

pub struct Errors {
}

#[derive(PartialEq, Properties)]
pub struct ErrorsProps {
    pub errors: Vec<(usize, String, bool)>,
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
                { ctx.props().errors.iter().map(|(idx, error, is_fading_out)| {
                    html!{
                        <div class={classes!("error", if *is_fading_out {"fade-out"} else {"fade-in"})} onclick={
                            let idx = *idx;
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
