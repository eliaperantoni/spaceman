use std::cell::RefCell;
use std::rc::Rc;
use std::thread::spawn;

use blossom_types::endpoint::Endpoint;
use blossom_types::repo::{RepoView, MethodView, ServiceView};
use blossom_types::callopout::CallOpOut;

use futures::{SinkExt, StreamExt};
use serde_json::to_string;
use web_sys::console::{error_1, log_1};
use web_sys::HtmlTextAreaElement;
use js_sys::{JsString, Reflect};
use futures::channel::mpsc;
use yew::platform::spawn_local;
use yew::prelude::*;

mod call;
mod commands;
mod glue;
mod components;

use components::pane::Pane;
use components::button::{Button, ButtonKind};
use components::repo::Repo;

use commands::*;
use call::*;

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
    set_input: Callback<(usize, String)>,

    send_msg: Callback<UiMsg>,
}

enum MainMsg {
    SelectTab(usize),
    DestroyTab(usize),
    SetInput((usize, String)),
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
            MainMsg::SelectTab(tab_index) => {
                ctx.props().select_tab.emit(tab_index);
                false
            },
            MainMsg::DestroyTab(tab_index) => {
                ctx.props().destroy_tab.emit(tab_index);
                false
            },
            MainMsg::SetInput((tab_index, input)) => {
                ctx.props().set_input.emit((tab_index, input));
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
                    {
                    if let Some(active_tab) = ctx.props().active_tab.clone() {
                        let tab = &ctx.props().tabs[active_tab];
                        let output = tab.selected_output.map(|selected_output| {
                            tab.output.get(selected_output).cloned().unwrap()
                        }).unwrap_or_else(|| String::new());
                        html!{
                            <>
                                <div class="header">
                                    {
                                        if tab.method.is_client_streaming {
                                            if let Some(call_id) = tab.call_id {
                                                let send_msg = ctx.props().send_msg.clone();

                                                let input = tab.input.clone();

                                                let onclick_send = {
                                                    let send_msg = send_msg.clone();
                                                    move |_| {
                                                        send_msg.emit(UiMsg::CallSend { call_id, message: input.clone() });
                                                    }
                                                };
                                                let onclick_commit = {
                                                    let send_msg = send_msg.clone();
                                                    move |_| {
                                                        send_msg.emit(UiMsg::CallCommit { call_id });
                                                    }
                                                };
                                                let onclick_stop = {
                                                    let send_msg = send_msg.clone();
                                                    move |_| {
                                                        send_msg.emit(UiMsg::CallCancel { call_id });
                                                    }
                                                };
                                                html!{
                                                    <>
                                                        <Button text="Send" kind={ ButtonKind::Blue } onclick={ onclick_send }/>
                                                        <Button text="Commit" kind={ ButtonKind::Green } onclick={ onclick_commit }/>
                                                        <Button text="Stop" kind={ ButtonKind::Red } onclick={ onclick_stop }/>
                                                    </>
                                                }
                                            } else {
                                                let send_msg = ctx.props().send_msg.clone();

                                                let method_full_name = tab.method.full_name.clone();
                                                let input = tab.input.clone();

                                                let onclick = move |_| {
                                                    send_msg.emit(UiMsg::CallStart {
                                                        tab_index: active_tab,
                                                        method_full_name: method_full_name.clone(),
                                                        initial_message: Some(input.clone()),
                                                    });
                                                };
                                                html!{
                                                    <Button text="Start" kind={ ButtonKind::Green } { onclick }/>
                                                }
                                            }
                                        } else {
                                            if let Some(call_id) = tab.call_id {
                                                let send_msg = ctx.props().send_msg.clone();

                                                let onclick = move |_| {
                                                    send_msg.emit(UiMsg::CallCancel { call_id });
                                                };
                                                html!{
                                                    <Button text="Stop" kind={ ButtonKind::Red } { onclick }/>
                                                }
                                            } else {
                                                let send_msg = ctx.props().send_msg.clone();

                                                let method_full_name = tab.method.full_name.clone();
                                                let input = tab.input.clone();

                                                let onclick = move |_| {
                                                    send_msg.emit(UiMsg::CallStart {
                                                        tab_index: active_tab,
                                                        method_full_name: method_full_name.clone(),
                                                        initial_message: Some(input.clone()),
                                                    });
                                                };
                                                html!{
                                                    <Button text="Run" kind={ ButtonKind::Green } { onclick }/>
                                                }
                                            }
                                        }
                                    }
                                </div>
                                <Pane initial_left={ 0.5 }>
                                    <textarea value={ tab.input.clone() } oninput={ ctx.link().callback(move |ev: InputEvent| MainMsg::SetInput((active_tab, ev.target_unchecked_into::<HtmlTextAreaElement>().value()))) }/>
                                    <textarea value={ output } />
                                </Pane>
                            </>
                        }
                    } else {
                        html!{
                            <></>
                        }
                    }
                    }
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

    input: String,
    output: Vec<String>,
    selected_output: Option<usize>,

    follow_output: bool,

    call_id: Option<i32>,
}

impl Tab {
    pub fn new(method: MethodView, input: String) -> Self {
        Self {
            method,
            input,
            output: Vec::new(),
            selected_output: None,
            follow_output: true,
            call_id: None,
        }
    }
}

enum UiMsg {
    // For changing the list of files to load
    SetProtoFiles(Vec<String>),
    // For changing the loaded RepoView, should be the result of a
    // UiMsg::SetProtoFiles
    SetRepoView(RepoView),
    ReportError(String),
    RequestNewTab{
        // Index of service in RepoView
        service_idx: usize,
        // Index of method in RepoView
        method_idx: usize,
    },
    NewTab{
        method_view: MethodView,
        input: String,
    },
    SelectTab(usize),
    DestroyTab(usize),
    SetInput((usize, String)),

    // Sets the tab's call_id and bootstraps the request. Once listen and
    // start_call (which are asynchronous) resolve, they register the listener
    // using CallStarted
    CallStart {
        // To set the call_id of the tab so that we can show that the request is
        // inflight
        tab_index: usize,
        method_full_name: String,
        initial_message: Option<String>,
    },
    CallStarted {
        // We use the call_id to uniquely identify a request because the tab
        // index might have changed
        call_id: i32,
        listener: Listener,
    },
    CallSend {
        // We use the call_id to uniquely identify a request because the tab
        // index might have changed
        call_id: i32,
        message: String,
    },
    CallCommit {
        // We use the call_id to uniquely identify a request because the tab
        // index might have changed
        call_id: i32,
    },
    CallCancel {
        // We use the call_id to uniquely identify a request because the tab
        // index might have changed
        call_id: i32,
    },
    CallRecv {
        // We use the call_id to uniquely identify a request because the tab
        // index might have changed
        call_id: i32,
        op_out: CallOpOut,
    },
}

struct Ui {
    // Shown on the sidebar
    repo_view: Option<RepoView>,

    tabs: Vec<(Tab, Option<Listener>)>,
    active_tab: Option<usize>,

    next_call_id: i32,
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

            next_call_id: 1,
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
            UiMsg::RequestNewTab { service_idx, method_idx } => {
                let repo_view = self.repo_view.as_ref().expect("to have a repo view, since a method button was pressed");
                let method_view = repo_view.services.get(service_idx).and_then(|service| service.methods.get(method_idx));
                if let Some(method_view) = method_view {
                    let method_view = method_view.clone();
                    ctx.link().send_future(async {
                        let input = get_empty_input_message(&method_view.full_name).await;
                        UiMsg::NewTab { method_view, input: input.ok().unwrap_or_else(|| String::new()) }
                    });
                }
                false
            },
            UiMsg::NewTab{method_view, input} => {
                self.tabs.push((Tab::new(method_view, input), None));
                self.active_tab = Some(self.tabs.len() - 1);
                true
            },
            UiMsg::SelectTab(tab_index) => {
                self.active_tab = Some(tab_index);
                true
            },
            UiMsg::DestroyTab(tab_index) => {
                if let Some(active_tab) = self.active_tab {
                    if self.tabs.len() == 1 {
                        self.active_tab = None;
                    } else if active_tab >= tab_index {
                        self.active_tab = Some(active_tab - 1);
                    }
                }
                self.tabs.remove(tab_index);
                true
            },
            UiMsg::SetInput((tab_index, input)) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.input = input;
                true
            },
            UiMsg::CallStart { tab_index, method_full_name, initial_message } => {
                let call_id = self.next_call_id;
                self.next_call_id += 1;

                let (tab, _) = &mut self.tabs[tab_index];
                tab.call_id = Some(call_id);

                let recv = ctx.link().callback(move |op_out| {
                    UiMsg::CallRecv { call_id, op_out }
                });

                ctx.link().send_future(async move {
                    let listener = listen(call_id, Box::new(move |op_out| {
                        recv.emit(op_out);
                    })).await;
                    start_call(call_id, &Endpoint{
                        authority: "localhost:7575".to_string(),
                        tls: None,
                    }, &method_full_name, &[]).await.unwrap();
                    if let Some(initial_message) = initial_message {
                        message(call_id, &initial_message);
                    }
                    UiMsg::CallStarted { call_id, listener }
                });
                false
            },
            UiMsg::CallStarted { call_id, listener } => {
                let (tab, tab_listener) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)).unwrap();
                tab.output.clear();
                tab.selected_output = None;
                *tab_listener = Some(listener);
                true
            },
            UiMsg::CallSend { call_id, message: body } => {
                if let Some((tab, _)) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)) {
                    if let Some(call_id) = tab.call_id {
                        message(call_id, &body);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            UiMsg::CallRecv { call_id, op_out } => {
                let (tab, tab_listener) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)).unwrap();
                match op_out {
                    CallOpOut::Msg(output) => {
                        tab.output.push(output);

                        if !tab.method.is_server_streaming {
                            tab.selected_output = Some(0);
                            terminate_call((tab, tab_listener));
                        } else if tab.follow_output || tab.output.len() == 1 {
                            tab.selected_output = Some(tab.output.len() - 1);
                        }
                    },
                    CallOpOut::Err(err) => {
                        ctx.link().send_message(UiMsg::ReportError(err));
                        // Abort the request as soon as we encounter an error
                        terminate_call((tab, tab_listener));
                    },
                    CallOpOut::InvalidInput => {
                        ctx.link().send_message(UiMsg::ReportError("Badly formatted input message".to_string()));
                        if !tab.method.is_client_streaming {
                            // Technically the backend is still waiting for one
                            // correctly formatted message but we're not going
                            // to send any. Abort the call.
                            cancel(tab.call_id.expect("to receive a CallOpOut::InvalidInput only for an ongoing call"));
                            terminate_call((tab, tab_listener));
                        }
                    },
                    CallOpOut::InvalidOutput => {
                        ctx.link().send_message(UiMsg::ReportError("Badly formatted output message".to_string()));
                        if !tab.method.is_server_streaming {
                            // This was the only message that we were ever going
                            // to receive, too bad it wasn't in the right format
                            terminate_call((tab, tab_listener));
                        }
                    },
                    CallOpOut::Commit => {
                        terminate_call((tab, tab_listener));
                    }
                }
                true
            },
            UiMsg::CallCancel { call_id } => {
                if let Some((tab, tab_listener)) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)) {
                    if let Some(call_id) = tab.call_id {
                        cancel(call_id);
                        terminate_call((tab, tab_listener));
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            UiMsg::CallCommit { call_id } => {
                if let Some((tab, _)) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)) {
                    if let Some(call_id) = tab.call_id {
                        commit(call_id);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => todo!()

        //     UiMsg::CallUnary { tab_index, method_full_name, input } => {
        //         let call_id = self.next_call_id;
        //         self.next_call_id += 1;

        //         self.tabs[tab_index].call_id = Some(call_id);
        //         ctx.link().send_future(async move {
        //             let (tx, mut rx) = mpsc::channel(1);
        //             let tx = Rc::new(RefCell::new(tx));
        //             let lis = listen(call_id, Box::new(move |call_op_out| {
        //                 let tx = tx.clone();
        //                 spawn_local(async move {
        //                     tx.borrow_mut().send(call_op_out).await.unwrap();
        //                 });
        //             })).await;
        //             start_call(call_id, &Endpoint{
        //                 authority: "localhost:7575".to_string(),
        //                 tls: None,
        //             }, &method_full_name, &[]).await.unwrap();
        //             message(call_id, &input);
        //             let output = rx.next().await.unwrap();
        //             UiMsg::EndUnary { call_id, output : match output {
        //                 CallOpOut::Msg(output) => output,
        //                 CallOpOut::InvalidInput => String::from("Input message is invalid"),
        //                 CallOpOut::InvalidOutput => String::from("Received invalid output message"),
        //                 CallOpOut::Err(err) => format!("Error: {}", &err),
        //                 _ => unreachable!()
        //             }}  
        //         });
        //         true
        //     },
        //     UiMsg::EndUnary { call_id, output } => {
        //         let target_tab = self.tabs.iter_mut().find(move |tab| tab.call_id == Some(call_id));
        //         if let Some(target_tab) = target_tab {
        //             target_tab.output.clear();
        //             target_tab.output.push(output);
        //             target_tab.call_id = None;
        //             true
        //         } else {
        //             false
        //         }
        //     },

        //     UiMsg::CallServerStreaming { tab_index, method_full_name, input } => {
        //         let call_id = self.next_call_id;
        //         self.next_call_id += 1;

        //         self.tabs[tab_index].call_id = Some(call_id);

        //         let recv = ctx.link().callback(move|op_out| {
        //             UiMsg::ServerStreamRecv { call_id, op_out }
        //         });

        //         spawn_local(async move {
        //             let unlisten =listen(call_id, Box::new(move |op_out| {
        //                 recv.emit(op_out);
        //             })).await;
        //             start_call(call_id, &Endpoint{
        //                 authority: "localhost:7575".to_string(),
        //                 tls: None,
        //             }, &method_full_name, &[]).await.unwrap();
        //         });

        //         true
        //     },
        //     UiMsg::ServerStreamRecv { call_id, op_out } => {
        //         let target_tab = self.tabs.iter_mut().find(move |tab| tab.call_id == Some(call_id));
        //         if let Some(target_tab) = target_tab {
        //             match op_out {
        //                 CallOpOut::Msg(output) => {
        //                     target_tab.output.clear();
        //                     target_tab.output.push(output);
        //                 },
        //                 _ => unreachable!()
        //             };
        //             true
        //         } else {
        //             false
        //         }
        //     },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_new_tab = ctx.link().callback(|(service_idx, method_idx)| {
            UiMsg::RequestNewTab{service_idx, method_idx}
        });

        let select_tab = ctx.link().callback(|idx| {
            UiMsg::SelectTab(idx)
        });

        let destroy_tab = ctx.link().callback(|idx: usize| {
            UiMsg::DestroyTab(idx)
        });

        let set_input = ctx.link().callback(|(idx, input)| {
            UiMsg::SetInput((idx, input))
        });

        let send_msg = ctx.link().callback(|msg: UiMsg| {
            msg
        });

        let tabs: Vec<_> = self.tabs.iter().map(|(tab, _)| tab.clone()).collect();

        html! {
            <div class="ui">
                <Pane initial_left={ 0.2 }>
                    <Sidebar repo_view={ self.repo_view.clone() } { on_new_tab }/>
                    <Main { tabs } active_tab={ self.active_tab } { select_tab } { destroy_tab } { set_input } { send_msg }/>
                </Pane>
            </div>
        }
    }
}

fn terminate_call(tab: (&mut Tab, &mut Option<Listener>)) {
    let (tab, listener) = tab;
    tab.call_id = None;
    let _ = listener.take();
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<Ui>::new().render();
}
