use blossom_types::repo::{RepoView, MethodView};
use serde_json::to_string;
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
    repo_view: Option<RepoView>,
    on_new_tab: Callback<(usize, usize)>,
}

#[function_component]
fn Sidebar(props: &SidebarProps) -> Html {
    html! {
        <div class="sidebar">
            <Button
                text="Settings"
                icon="img/cog.svg"/>
            <Repo repo_view={ props.repo_view.clone() } on_new_tab={ props.on_new_tab.clone() }/>
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct MainProps {
    tabs: Vec<Tab>,
    active_tab: Option<usize>,
}

enum MainMsg {}

struct Main {}

impl Component for Main {
    type Message = MainMsg;
    type Properties = MainProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="main">
                <div class="tabs">
                    {for ctx.props().tabs.iter().enumerate().map(|(idx, tab)| html! {
                        <div class={ classes!("tab", ctx.props().active_tab.filter(|active_tab| *active_tab == idx).and(Some("active"))) }>
                            <div class="name">{ tab.method.name.clone() }</div>
                            <div class="close">
                                <img src="img/close.svg"/>
                            </div>
                        </div>
                    })}
                </div>
                <Pane initial_left={ 0.5 }>
                    <div></div>
                    <div></div>
                </Pane>
            </div>
        }
    }
}

#[derive(PartialEq, Clone)]
struct Tab {
    // FIXME
    // This wouldn't change when the protos are reloaded. So we could be making
    // a call to a method that the backend doesn't know. The serial contained
    // inside the MethodView may no longer be valid when the protos are reloaded.
    // How do we solve this?
    method: MethodView,
}

enum UiMsg {
    // For changing the list of files to load
    SetProtoFiles(Vec<String>),
    // For changing the loaded RepoView, should be the result of a
    // UiMsg::SetProtoFiles
    SetRepoView(RepoView),
    ReportError(String),
    NewTab{
        // Index of service in RepoView
        service_idx: usize,
        // Index of method in RepoView
        method_idx: usize,
    },
}

struct Ui {
    // Shown on the sidebar
    repo_view: Option<RepoView>,

    tabs: Vec<Tab>,
    active_tab: Option<usize>,
}

impl Component for Ui {
    type Properties = ();
    type Message = UiMsg;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(UiMsg::SetProtoFiles(vec![
            "/home/elia/code/blossom/playground/proto/playground.desc".to_string(),
        ]));
        Self {
            repo_view: None,
            tabs: Vec::new(),
            active_tab: None,
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
            },
            UiMsg::NewTab{service_idx, method_idx} => {
                let repo_view = self.repo_view.as_ref().expect("to have a repo view, since a method button was pressed");
                let method = repo_view.services.get(service_idx).and_then(|service| service.methods.get(method_idx));

                if let Some(method) = method {
                    self.tabs.push(Tab {
                        method: method.clone(),
                    });
                    true
                } else {
                    false
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_new_tab = ctx.link().callback(|(service_idx, method_idx)| {
            UiMsg::NewTab{service_idx, method_idx}
        });

        html! {
            <div class="ui">
                <Pane initial_left={ 0.2 }>
                    <Sidebar repo_view={ self.repo_view.clone() } { on_new_tab }/>
                    <Main tabs={ self.tabs.clone() } active_tab={ self.active_tab }/>
                </Pane>
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<Ui>::new().render();
}
