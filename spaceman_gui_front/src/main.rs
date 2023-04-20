use std::cell::RefCell;
use std::rc::Rc;
use std::thread::spawn;

use spaceman_types::endpoint::Endpoint;
use spaceman_types::repo::{RepoView, MethodView, ServiceView};
use spaceman_types::callopout::CallOpOut;
use spaceman_types::settings::{Settings, Profile};

use futures::{SinkExt, StreamExt};
use serde_json::to_string;
use web_sys::console::{error_1, log_1};
use web_sys::{HtmlTextAreaElement, HtmlElement};
use web_sys::HtmlInputElement;
use js_sys::{JsString, Reflect};
use futures::channel::mpsc;
use yew::platform::spawn_local;
use yew::prelude::*;
use gloo_timers::callback::Timeout;
use slab::Slab;
use uuid::Uuid;

mod call;
mod commands;
mod glue;
mod components;

use components::pane::Pane;
use components::button::{Button, ButtonKind};
use components::repo::Repo;
use components::metadata_editor::MetadataEditor;
use components::errors::Errors;

use commands::*;
use call::*;

use crate::components::settings::SettingsEditor;

#[derive(PartialEq, Properties)]
struct SidebarProps {
    repo_view: Option<RepoView>,
    send_msg: Callback<UiMsg>,
}

#[function_component]
fn Sidebar(props: &SidebarProps) -> Html {
    html! {
        <div class="sidebar">
            <Button
                onclick={ props.send_msg.clone().reform(|_| UiMsg::GoToSettings) }
                text="Settings"
                icon="img/cog.svg"/>
            <Repo
                repo_view={ props.repo_view.clone() }
                on_new_tab={ props.send_msg.clone().reform(|(service_idx, method_idx)| UiMsg::RequestNewTab { service_idx, method_idx }) }/>
        </div>
    }
}

#[derive(PartialEq, Properties)]
struct MainProps {
    tabs: Vec<Tab>,
    active_tab: Option<usize>,
    send_msg: Callback<UiMsg>,
    profiles: Vec<(Uuid, String)>,
}

enum MainMsg {
    SelectTab(usize),
    DestroyTab(usize),
}

struct Main {
    input_textarea: NodeRef,
    output_textarea: NodeRef,
}

impl Component for Main {
    type Message = MainMsg;
    type Properties = MainProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            input_textarea: NodeRef::default(),
            output_textarea: NodeRef::default()
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            MainMsg::SelectTab(tab_index) => {
                ctx.props().send_msg.emit(UiMsg::SelectTab(tab_index));
                false
            },
            MainMsg::DestroyTab(tab_index) => {
                ctx.props().send_msg.emit(UiMsg::DestroyTab(tab_index));
                false
            }
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            spawn_local(glue::initMonaco(
                self.input_textarea.cast::<HtmlElement>().unwrap().into(),
                "input",
                false,
            ));
            spawn_local(glue::initMonaco(
                self.output_textarea.cast::<HtmlElement>().unwrap().into(),
                "output",
                true,
            ));
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
                    if let Some(active_tab) = ctx.props().active_tab.clone() {
                        <div class="header">
                            // BEGIN PROFILES
                            <select
                                class="select"
                                key={active_tab}
                                onchange={
                                    ctx.props().send_msg.clone().reform(move |ev: Event| {
                                        let val = ev.target_unchecked_into::<HtmlInputElement>().value();
                                        UiMsg::UseProfile(active_tab, Uuid::parse_str(val.as_str()).ok())
                                    })
                                }>
                                <option
                                    value=""
                                    selected={ ctx.props().tabs[active_tab].profile_id.is_none() }>
                                    { "" }
                                </option>
                                {
                                    ctx.props().profiles.iter().map(|(id, profile_name)| {
                                        html! {
                                            <option
                                                value={ id.to_string() }
                                                selected={ ctx.props().tabs[active_tab].profile_id == Some(id.clone()) }>
                                                { profile_name.clone() }
                                            </option>
                                        }
                                    }).collect::<Html>()
                                }
                            </select>
                            // END PROFILES

                            // BEGIN COMMANDS
                            {{ // I have no idea why double braces are necessary here tbh
                                let tab = &ctx.props().tabs[active_tab];

                                if tab.method.is_client_streaming {
                                    if let Some(call_id) = tab.call_id {
                                        let input_monaco_id = tab.input_monaco_id;
                                        html!{
                                            <>
                                                <Button
                                                    text="Send"
                                                    kind={ ButtonKind::Blue }
                                                    onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                        UiMsg::CallSend { call_id, message: String::from(glue::monacoRead("input", input_monaco_id)) }
                                                    })}/>
                                                <Button
                                                    text="Commit"
                                                    kind={ ButtonKind::Green }
                                                    onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                        UiMsg::CallCommit { call_id }
                                                    })}/>
                                                <Button
                                                    text="Stop"
                                                    kind={ ButtonKind::Red }
                                                    onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                        UiMsg::CallCancel { call_id }
                                                    })}/>
                                            </>
                                        }
                                    } else {
                                        let method_full_name = tab.method.full_name.clone();
                                        let input_monaco_id = tab.input_monaco_id;
                                        html!{
                                            <Button
                                                text="Start"
                                                kind={ ButtonKind::Green }
                                                onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                    UiMsg::CallStart {
                                                        tab_index: active_tab,
                                                        method_full_name: method_full_name.clone(),
                                                        initial_message: Some(String::from(glue::monacoRead("input", input_monaco_id))),
                                                    }
                                                })}/>
                                        }
                                    }
                                } else {
                                    if let Some(call_id) = tab.call_id {
                                        html!{
                                            <Button
                                                text="Stop"
                                                kind={ ButtonKind::Red }
                                                onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                    UiMsg::CallCancel { call_id }
                                                })}/>
                                        }
                                    } else {
                                        let method_full_name = tab.method.full_name.clone();
                                        let input_monaco_id = tab.input_monaco_id;
                                        html!{
                                            <Button
                                                text="Run"
                                                kind={ ButtonKind::Green }
                                                onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                    UiMsg::CallStart {
                                                        tab_index: active_tab,
                                                        method_full_name: method_full_name.clone(),
                                                        initial_message: Some(String::from(glue::monacoRead("input", input_monaco_id))),
                                                    }
                                                })}/>
                                        }
                                    }
                                }
                            }}
                            // END COMMANDS
                        </div>
                    }
                    <Pane initial_left={ 0.5 }>
                        <div class="main-pane-col">
                            <div 
                                class={
                                    classes!(
                                        "editor-wrapper",
                                        ctx.props().active_tab
                                            .and_then(|active_tab| if ctx.props().tabs[active_tab].editing_metadata {
                                                Some("hidden")
                                            } else {
                                                None
                                            })
                                    )
                                }>
                                <div ref={ self.input_textarea.clone() } class="editor"></div>
                            </div>
                            if let Some(active_tab) = ctx.props().active_tab.clone() {
                                if ctx.props().tabs[active_tab].editing_metadata {
                                    <MetadataEditor
                                        rows={ ctx.props().tabs[active_tab].metadata.clone() }
                                        new_row={ctx.props().send_msg.clone().reform(move |_|{
                                            UiMsg::NewMetadataRow(active_tab)
                                        })}
                                        update_row={ctx.props().send_msg.clone().reform(move |(row_idx, row)|{
                                            UiMsg::UpdateMetadataRow((active_tab, row_idx, row))
                                        })}
                                        delete_row={ctx.props().send_msg.clone().reform(move |row_idx|{
                                            UiMsg::DeleteMetadataRow((active_tab, row_idx))
                                        })}/>
                                }
                                <div class="bottom-line">
                                    <Button
                                        onclick={ctx.props().send_msg.clone().reform(move |_| {
                                            UiMsg::ToggleEditingMetadata(active_tab)
                                        })}
                                        has_led={true}
                                        is_led_lit={ctx.props().tabs[active_tab].editing_metadata}
                                        class={classes!("metadata-button")}
                                        text="Metadata"
                                        icon="img/agenda.svg"/>
                                </div>
                            }
                        </div>
                        <div class="main-pane-col">
                            <div class="editor-wrapper">
                                <div ref={ self.output_textarea.clone() } class="editor"></div>
                            </div>
                            if let Some(active_tab) = ctx.props().active_tab.clone() {
                                if ctx.props().tabs[active_tab].method.is_server_streaming && ctx.props().tabs[active_tab].output_monaco_ids.len() > 0 {
                                    <div class="bottom-line">
                                        <Button
                                            onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                UiMsg::ToggleFollowOutput(active_tab)
                                            })}
                                            has_led={true}
                                            is_led_lit={ctx.props().tabs[active_tab].follow_output}
                                            class={classes!("follow")}
                                            text="Follow"/>
                                        <Button
                                            onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                UiMsg::NavigateOutput((active_tab, -1))
                                            })}
                                            class={classes!("prev")}
                                            text="Prev"
                                            kind={ButtonKind::Cyan}/>
                                        <div class="counter">
                                            <span class="current">{ctx.props().tabs[active_tab].selected_output.map(|i| i + 1).unwrap_or(0)}</span>
                                            <img class="line" src="img/line.svg"/>
                                            <span class="of">{ctx.props().tabs[active_tab].output_monaco_ids.len()}</span>
                                        </div>
                                        <Button
                                            onclick={ctx.props().send_msg.clone().reform(move |_| {
                                                UiMsg::NavigateOutput((active_tab,1))
                                            })}
                                            class={classes!("next")}
                                            text="Next"
                                            kind={ButtonKind::Cyan}/>
                                    </div>
                                }
                            }
                        </div>
                    </Pane>
                </div>
            </div>
        }
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct MetadataRow {
    pub key: String,
    pub val: String,
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

    input_monaco_id: i32,
    output_monaco_ids: Vec<i32>,
    selected_output: Option<usize>,

    follow_output: bool,

    metadata: Vec<MetadataRow>,
    editing_metadata: bool,

    call_id: Option<i32>,

    profile_id: Option<Uuid>,
}

impl Tab {
    pub fn new(method: MethodView, input_monaco_id: i32) -> Self {
        Self {
            method,
            input_monaco_id,
            output_monaco_ids: Vec::new(),
            selected_output: None,
            follow_output: true,
            metadata: Vec::new(),
            editing_metadata: false,
            call_id: None,
            profile_id: None,
        }
    }
}

enum UiMsg {
    // For changing the list of files to load
    ReloadProtos,
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
    
    NavigateOutput((usize, i32)),
    ToggleFollowOutput(usize),
    ToggleEditingMetadata(usize),

    NewMetadataRow(usize),
    UpdateMetadataRow((usize, usize, MetadataRow)),
    DeleteMetadataRow((usize, usize)),

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
    CallStoreListener {
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
    
    DismissError(usize),
    DismissErrorPostAnimation(usize),

    SetSettings(Settings),
    GoToSettings,
    LeaveSettings,

    UseProfile(usize, Option<Uuid>),
}

struct Error {
    msg: String,
    is_fading_out: bool,
    timeout_for_dismiss: Option<Timeout>,
    timeout_for_removal_post_animation: Option<Timeout>,
}

struct Ui {
    // Shown on the sidebar
    repo_view: Option<RepoView>,

    tabs: Vec<(Tab, Option<Listener>)>,
    active_tab: Option<usize>,

    next_call_id: i32,

    errors: Slab<Error>,

    settings: Settings,
    is_in_settings: bool,
}

impl Component for Ui {
    type Properties = ();
    type Message = UiMsg;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future_batch(async {
            let mut messages = vec![UiMsg::ReloadProtos];
            let settings = load_settings().await;
            match settings {
                Ok(Some(settings)) => {
                    messages.insert(0, UiMsg::SetSettings(settings));
                },
                Ok(None) => {},
                Err(err) => {
                    messages.push(UiMsg::ReportError(err));
                }
            };
            messages
        });
        Self {
            repo_view: None,
            tabs: Vec::new(),
            active_tab: None,

            next_call_id: 1,

            errors: Slab::new(),

            settings: Settings::default(),
            is_in_settings: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UiMsg::ReloadProtos => {
                let paths = self.settings.proto_paths.clone();
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
                self.tabs.retain_mut(|(tab, _)| {
                    // Only keep tabs that are running and those whose method
                    // still exists in the new loaded proto repo
                    if tab.call_id.is_some() {
                        return true;
                    }
                    if let Some(updated_method_view) = repo_view.find_method_desc(&tab.method.full_name) {
                        tab.method = updated_method_view;
                        true
                    } else {
                        false
                    }
                });
                if let Some(active_tab) = self.active_tab {
                    if active_tab >= self.tabs.len() {
                        self.active_tab = Some(self.tabs.len() - 1);
                    }
                }
                self.repo_view = Some(repo_view);
                true
            },
            UiMsg::ReportError(err) => {
                let err_idx = self.errors.insert(Error {
                    msg: err,
                    is_fading_out: false,
                    timeout_for_dismiss: None,
                    timeout_for_removal_post_animation: None,
                });
                let dismiss = ctx.link().callback(move |_| {
                    UiMsg::DismissError(err_idx)
                });
                let timeout = Timeout::new(10_000, move || {
                    dismiss.emit(());
                });
                self.errors[err_idx].timeout_for_dismiss = Some(timeout);
                true
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
                let monaco_input = glue::monacoAddTab("input");
                glue::monacoGoToTab("input", monaco_input);
                glue::monacoWrite("input", monaco_input, &input);

                glue::monacoDeselect("output");

                let mut tab = Tab::new(method_view, monaco_input);

                if let Some(initial_profile_id) = 
                    self.settings.profiles
                        .iter()
                        .min_by_key(|(_, profile)| profile.ordinal)
                        .and_then(|(id, _)| Some(id.clone()))
                {
                    tab.profile_id = Some(initial_profile_id);
                }

                self.tabs.push((tab, None));
                self.active_tab = Some(self.tabs.len() - 1);
                true
            },
            UiMsg::SelectTab(tab_index) => {
                let (tab, _) = &self.tabs[tab_index];
                glue::monacoGoToTab("input", tab.input_monaco_id);
                if let Some(selected_output) = tab.selected_output {
                    glue::monacoGoToTab("output", tab.output_monaco_ids[selected_output]);
                } else {
                    glue::monacoDeselect("output");
                }
                self.active_tab = Some(tab_index);
                true
            },
            UiMsg::DestroyTab(tab_index) => {
                if let Some(active_tab) = self.active_tab {
                    if self.tabs.len() == 1 {
                        glue::monacoDeselect("input");
                        glue::monacoDeselect("output");
                        self.active_tab = None;
                    } else {
                        let goto_index;
                        if active_tab == tab_index  {
                            if active_tab != self.tabs.len() - 1 {
                                goto_index = active_tab + 1;
                            } else {
                                goto_index = active_tab - 1;
                                self.active_tab = Some(goto_index);
                            }
                        } else if active_tab == tab_index + 1 {
                            goto_index = active_tab;
                            self.active_tab = Some(active_tab - 1);
                        } else {
                            goto_index = active_tab - 1;
                            self.active_tab = Some(active_tab - 1);
                        }
    
                        let (goto_tab, _) = &self.tabs[goto_index];
    
                        glue::monacoGoToTab("input", goto_tab.input_monaco_id);
                        if let Some(selected_output) = goto_tab.selected_output {
                            glue::monacoGoToTab("output", goto_tab.output_monaco_ids[selected_output]);
                        } else {
                            glue::monacoDeselect("output");
                        }
                    }
                }

                let (tab, _) = &mut self.tabs[tab_index];
                glue::monacoDelTab("input", tab.input_monaco_id);
                for monaco_id in &tab.output_monaco_ids {
                    glue::monacoDelTab("output", *monaco_id);
                }

                self.tabs.remove(tab_index);
                true
            },
            UiMsg::NavigateOutput((tab_index, move_by)) => {
                let (tab, _) = &mut self.tabs[tab_index];
                let n_outputs = tab.output_monaco_ids.len();
                if let Some(selected_output) = tab.selected_output.as_mut() {
                    let set_to = *selected_output as i32 + move_by;
                    if set_to >= 0 && (set_to as usize) < n_outputs {
                        *selected_output = set_to as usize;
                        glue::monacoGoToTab("output", tab.output_monaco_ids[*selected_output]);
                    }
                }
                true
            },
            UiMsg::ToggleFollowOutput(tab_index) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.follow_output = !tab.follow_output;
                if tab.follow_output && tab.output_monaco_ids.len() > 0 {
                    let index_of_last = tab.output_monaco_ids.len() - 1;
                    tab.selected_output = Some(index_of_last);
                    glue::monacoGoToTab("output", tab.output_monaco_ids[index_of_last]);
                }
                true
            },
            UiMsg::ToggleEditingMetadata(tab_index) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.editing_metadata = !tab.editing_metadata;
                true
            },
            UiMsg::CallStart { tab_index, method_full_name, initial_message } => {
                let (tab, _) = &mut self.tabs[tab_index];

                let profile_id = if let Some(profile_id) = tab.profile_id.clone() {
                    profile_id
                } else {
                    ctx.link().send_message(UiMsg::ReportError(String::from("You have to select a profile first")));
                    return true;
                };

                let profile = if let Some(profile) = self.settings.profiles.get(&profile_id).cloned() {
                    profile
                } else {
                    ctx.link().send_message(UiMsg::ReportError(String::from("Selected profile does not exist")));
                    return true;
                };

                let call_id = self.next_call_id;
                self.next_call_id += 1;

                tab.call_id = Some(call_id);

                for monaco_id in &tab.output_monaco_ids {
                    glue::monacoDeselect("output");
                    glue::monacoDelTab("output", *monaco_id);
                }
                tab.output_monaco_ids.clear();
                tab.selected_output = None;

                let metadata = tab.metadata.clone();

                let recv = ctx.link().callback(move |op_out| {
                    UiMsg::CallRecv { call_id, op_out }
                });

                ctx.link().send_future(async move {
                    let listener = listen(call_id, Box::new(move |op_out| {
                        recv.emit(op_out);
                    })).await;
                    start_call(call_id, &profile.endpoint, &method_full_name, &metadata[..]).await.unwrap();
                    if let Some(initial_message) = initial_message {
                        message(call_id, &initial_message);
                    }
                    UiMsg::CallStoreListener { call_id, listener }
                });
                false
            },
            UiMsg::CallStoreListener { call_id, listener } => {
                let (_, tab_listener) = self.tabs.iter_mut().find(move |(tab, _)| tab.call_id == Some(call_id)).unwrap();
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
                let (tab_index, (tab, tab_listener)) = self.tabs
                    .iter_mut()
                    .enumerate()
                    .find(move |(_, (tab, _))| tab.call_id == Some(call_id))
                    .unwrap();
                match op_out {
                    CallOpOut::Msg(output) => {
                        let monaco_id = glue::monacoAddTab("output");
                        glue::monacoWrite("output", monaco_id, &output);
                        tab.output_monaco_ids.push(monaco_id);

                        if !tab.method.is_server_streaming {
                            if self.active_tab == Some(tab_index) {
                                glue::monacoGoToTab("output", monaco_id);
                            }
                            tab.selected_output = Some(0);

                            terminate_call((tab, tab_listener));
                        } else if tab.follow_output || tab.output_monaco_ids.len() == 1 {
                            if self.active_tab == Some(tab_index) { 
                                glue::monacoGoToTab("output", monaco_id);
                            }
                            tab.selected_output = Some(tab.output_monaco_ids.len() - 1);
                        }
                    },
                    CallOpOut::Err(err) => {
                        ctx.link().send_message(UiMsg::ReportError(err));
                        // Abort the request as soon as we encounter an error
                        terminate_call((tab, tab_listener));
                    },
                    CallOpOut::InvalidInput(err) => {
                        ctx.link().send_message(UiMsg::ReportError(format!("Badly formatted input message: {err}")));
                        if !tab.method.is_client_streaming {
                            // Technically the backend is still waiting for one
                            // correctly formatted message but we're not going
                            // to send any. Abort the call.
                            cancel(tab.call_id.expect("to receive a CallOpOut::InvalidInput only for an ongoing call"));
                            terminate_call((tab, tab_listener));
                        }
                    },
                    CallOpOut::InvalidOutput(err) => {
                        ctx.link().send_message(UiMsg::ReportError(format!("Badly formatted output message: {err}")));
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
            },
            UiMsg::NewMetadataRow(tab_index) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.metadata.push(Default::default());
                true
            },
            UiMsg::UpdateMetadataRow((tab_index, row_index, row)) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.metadata[row_index] = row;
                true
            },
            UiMsg::DeleteMetadataRow((tab_index, row_index)) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.metadata.remove(row_index);
                true
            },
            UiMsg::DismissError(idx) => {
                self.errors[idx].is_fading_out = true;
                self.errors[idx].timeout_for_dismiss = None;
                let cb = ctx.link().callback(move |_| UiMsg::DismissErrorPostAnimation(idx));
                self.errors[idx].timeout_for_removal_post_animation = Some(Timeout::new(150, move || {
                    cb.emit(());
                }));
                true
            },
            UiMsg::DismissErrorPostAnimation(idx) => {
                self.errors.remove(idx);
                true
            },
            UiMsg::SetSettings(settings) => {
                self.settings = settings;
                true
            },
            UiMsg::GoToSettings => {
                self.is_in_settings = true;
                true
            },
            UiMsg::LeaveSettings => {
                self.is_in_settings = false;
                ctx.link().send_message(UiMsg::ReloadProtos);

                let settings = self.settings.clone();
                ctx.link().send_future_batch(async move {
                    if let Err(err) = save_settings(&settings).await {
                        Some(UiMsg::ReportError(err))
                    } else {
                        None
                    }
                });

                true
            },

            UiMsg::UseProfile(tab_index, profile_id) => {
                let (tab, _) = &mut self.tabs[tab_index];
                tab.profile_id = profile_id;
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let send_msg = ctx.link().callback(|msg: UiMsg| {
            msg
        });

        let tabs: Vec<_> = self.tabs.iter().map(|(tab, _)| tab.clone()).collect();

        html! {
            <div class="ui">
                if self.is_in_settings {
                    <SettingsEditor
                        settings={ self.settings.clone() }
                        leave_settings={ send_msg.clone().reform(|_| UiMsg::LeaveSettings) }
                        set_settings={ send_msg.clone().reform(|settings| UiMsg::SetSettings(settings)) }/>
                } else {
                    <Pane initial_left={ 0.2 }>
                        <Sidebar repo_view={ self.repo_view.clone() } send_msg={ send_msg.clone() }/>
                        <Main { tabs } active_tab={ self.active_tab } send_msg={ send_msg.clone() } profiles={{
                            let mut profiles = self.settings.profiles.iter().map(|(id, profile)| {
                                (id.clone(), profile.clone())
                            }).collect::<Vec<_>>();
                            profiles.sort_by_key(|(_, profile)| profile.ordinal);
                            profiles.into_iter().map(|(id, profile)| (id, profile.name)).collect::<Vec<_>>()
                        }}/>
                    </Pane>
                }
                <Errors errors={ self.errors.iter().map(|(idx, Error {msg, is_fading_out, ..})| {
                    (idx, msg.clone(), *is_fading_out)
                }).collect::<Vec<_>>() } dismiss_error={ ctx.link().callback(|idx| UiMsg::DismissError(idx)) } />
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
