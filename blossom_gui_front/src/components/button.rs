use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    pub text: String,
    pub icon: Option<AttrValue>,
    pub style: Option<AttrValue>,
}

#[function_component]
pub fn Button(props: &ButtonProps) -> Html {
    html!{
        <button class="button" style={ props.style.clone() }>
            {
                if let Some(icon) = props.icon.clone() {
                    html!{<img src={ icon }/>}
                } else {
                    html!{}
                }
            }
            { props.text.clone() }
        </button>
    }
}
