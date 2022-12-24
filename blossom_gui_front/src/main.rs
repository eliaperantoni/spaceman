use blossom_types::repo::RepoView;
use web_sys::console::error_1;
use js_sys::JsString;
use yew::prelude::*;

mod call;
mod commands;
mod glue;
mod components;

use components::pane::Pane;
use components::button::Button;
use components::repo::Repo;

use commands::*;

#[derive(PartialEq, Properties)]
struct SidebarProps {
    repo_view: Option<RepoView>
}

#[function_component]
fn Sidebar(props: &SidebarProps) -> Html {
    html! {
        <div class="sidebar">
            <Button
                text="Settings"
                icon="img/cog.svg"
                style="align-self: stretch"/>
            <Repo repo_view={ props.repo_view.clone() }/>
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

enum UiMsg {
    // For changing the list of files to load
    SetProtoFiles(Vec<String>),
    // For changing the loaded RepoView, should be the result of a
    // UiMsg::SetProtoFiles
    SetRepoView(RepoView),
    ReportError(String),
}

struct Ui {
    // Shown on the sidebar
    repo_view: Option<RepoView>,
}

impl Component for Ui {
    type Properties = ();
    type Message = UiMsg;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(UiMsg::SetProtoFiles(vec!["/home/elia/code/blossom/playground/proto/playground.desc".to_string()]));
        Self {
            repo_view: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UiMsg::SetProtoFiles(paths) => {
                ctx.link().send_future(async move {'fut: {
                    if let Err(err) = reset_repo().await {
                        break 'fut UiMsg::ReportError(err);
                    }
                    for path in &paths {
                        if let Err(err) = add_protobuf_descriptor(path).await {
                            break 'fut UiMsg::ReportError(err);
                        }
                    }
                    match get_repo_view().await {
                        Ok(repo_view) => UiMsg::SetRepoView(repo_view),
                        Err(err) => UiMsg::ReportError(err),
                    }
                }});
                false
            },
            UiMsg::SetRepoView(repo_view) => {
                self.repo_view = Some(repo_view);
                true
            },
            UiMsg::ReportError(err) => {
                error_1(&JsString::from(err));
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="ui">
                <Pane initial_left={ 0.2 } style_lhs="min-width: 150px">
                    <Sidebar repo_view={ self.repo_view.clone() }/>
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
