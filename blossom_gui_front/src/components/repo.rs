use yew::prelude::*;
use blossom_types::repo::RepoView;

#[derive(Properties, PartialEq)]
pub struct RepoProps {
    pub repo_view: Option<RepoView>,
}

#[function_component]
pub fn Repo(props: &RepoProps) -> Html {
    html! {
        <div>{ props.repo_view.is_some().to_string() }</div>
    }
}
