use yew::prelude::*;
use blossom_types::repo::{RepoView, ServiceView, MethodView};

#[derive(Properties, PartialEq)]
pub struct RepoProps {
    pub repo_view: Option<RepoView>,
}

#[function_component]
pub fn Repo(props: &RepoProps) -> Html {
    let content = if let Some(repo_view) = props.repo_view.clone() {
        repo_view.services.into_iter().map(|service_view| {
            html!{ <Service service_view={ service_view }/> }
        }).collect::<Html>()
    } else {
        html!{}
    };

    html! {
        <div class="repo">
            {content}
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct ServiceProps {
    service_view: ServiceView,
}

#[function_component]
fn Service(props: &ServiceProps) -> Html {
    let methods_n = props.service_view.methods.len();
    html! {
        <div class="service">
            <div class="name">{ props.service_view.full_name.clone() }</div>
            {
                props.service_view.methods.iter().enumerate().map(|(idx, method_view)| {
                    html!{ <Method method_view={ method_view.clone() } is_last={idx == methods_n - 1}/> }
                }).collect::<Html>()
            }
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct MethodProps {
    is_last: bool,
    method_view: MethodView,
}

#[function_component]
fn Method(props: &MethodProps) -> Html {
    html! {
        <div class="method">
            {
                if props.is_last {
                    html!{ <img class="uplink" src="img/method_uplink_last.svg"/> }
                } else {
                    html!{ <img class="uplink" src="img/method_uplink.svg"/> }
                }
            }
            <div class="name">{ props.method_view.name.clone() }</div>
        </div>
    }
}
