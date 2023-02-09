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
use components::button::{Button, ButtonKind};
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

    select_tab: Callback<usize>,
    destroy_tab: Callback<usize>,
}

enum MainMsg {
    SelectTab(usize),
    DestroyTab(usize),
}

struct Main {}

impl Component for Main {
    type Message = MainMsg;
    type Properties = MainProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            MainMsg::SelectTab(idx) => {
                ctx.props().select_tab.emit(idx);
                false
            },
            MainMsg::DestroyTab(idx) => {
                ctx.props().destroy_tab.emit(idx);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="main">
                <div class="tabs">
                    {for ctx.props().tabs.iter().enumerate().map(|(idx, tab)| html! {
                        <div class={ classes!("tab", ctx.props().active_tab.filter(|active_tab| *active_tab == idx).and(Some("active"))) }>
                            <div class="name" onclick={ ctx.link().callback(move |_| MainMsg::SelectTab(idx)) }>{ tab.method.name.clone() }</div>
                            <div class="close" onclick={ ctx.link().callback(move |_| MainMsg::DestroyTab(idx)) }>
                                <img src="img/close.svg"/>
                            </div>
                        </div>
                    })}
                </div>
                <div class="tab-content">
                    <div class="header">
                        <Button text="Run" kind={ ButtonKind::Green }/>
                    </div>
                    <Pane initial_left={ 0.5 }>
                        <textarea/>
                        <textarea/>
                    </Pane>
                </div>
            </div>
        }
    }
}

#[derive(PartialEq, Clone)]
struct Tab {
    // The full_name of the method inside here can be used to keep the tab
    // linked to the respective method even if the repo changes (files are
    // added, removed, or it is simply refreshed). In that scenario, the
    // MethodView of all tabs must be reloaded with the full_name acting as key.
    // Tabs whose method no longer exists in the repo should disappear from
    // the UI.
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
    SelectTab(usize),
    DestroyTab(usize),
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
            "/home/elia/code/proto/ono/logistics/server/ono_logistics_server.desc".to_string(),
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
                    // TODO Refresh tabs, setting a new MethodView to them.
                    // Discard those whose method's full_name doesn't match
                    // anything in the new repo view.
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
            },
            UiMsg::SelectTab(idx) => {
                self.active_tab = Some(idx);
                true
            },
            UiMsg::DestroyTab(idx) => {
                if let Some(active_tab) = self.active_tab {
                    if active_tab >= idx {
                        self.active_tab = Some(active_tab - 1);
                    }
                }
                self.tabs.remove(idx);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_new_tab = ctx.link().callback(|(service_idx, method_idx)| {
            UiMsg::NewTab{service_idx, method_idx}
        });

        let select_tab = ctx.link().callback(|idx| {
            UiMsg::SelectTab(idx)
        });

        let destroy_tab = ctx.link().callback(|idx: usize| {
            UiMsg::DestroyTab(idx)
        });

        html! {
            <div class="ui">
                <Pane initial_left={ 0.2 }>
                    <Sidebar repo_view={ self.repo_view.clone() } { on_new_tab }/>
                    <Main tabs={ self.tabs.clone() } active_tab={ self.active_tab } { select_tab } { destroy_tab }/>
                </Pane>
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<Ui>::new().render();
}
