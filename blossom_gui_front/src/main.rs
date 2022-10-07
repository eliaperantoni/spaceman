use std::future::Future;
use std::io::SeekFrom::End;

use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use web_sys::console::log_1;
use yew::prelude::*;

use blossom_types::repo::{MethodView, RepoView, Serial, ServiceView};
use blossom_types::endpoint::{Endpoint, TlsOptions};

use crate::command::*;

mod command;
mod call;
mod invoke;

enum UiMsg {
    SetDescriptorPath(String),
    AddProtobufDescriptor,
    SetRepoView(RepoView),
    SetInput(String),

    ResetOutputs,
    AddOutput(String),

    SelectMethod(Serial),

    SetAuthority(String),
    SetTLSEnable(bool),
    SetTLSNoCheck(bool),
    SetTLSCACert(String),

    SetMetadata(String),

    Call,

    Prev,
    Next,
}

struct Ui {
    new_descriptor_path: String,
    repo_view: Option<RepoView>,
    selected: Option<Serial>,

    tls_enabled: bool,
    endpoint: Endpoint,

    input: String,

    selected_output: usize,
    outputs: Vec<String>,

    metadata: String,
}

#[derive(PartialEq, Clone, Properties)]
struct MethodProperties {
    selected: Option<Serial>,
    select_method: Callback<Serial>,
    method: MethodView,
}

#[function_component(Method)]
fn method(props: &MethodProperties) -> Html {
    let select_method = props.select_method.clone();
    let serial = props.method.serial;

    let onclick = move |_| {
        select_method.emit(serial);
    };

    let class = if props.selected == Some(props.method.serial) {
        classes!["selected"]
    } else {
        classes![]
    };

    html! {
        <li {onclick} {class}>{ &props.method.name }</li>
    }
}

#[derive(PartialEq, Clone, Properties)]
struct ServiceProperties {
    selected: Option<Serial>,
    select_method: Callback<Serial>,
    service: ServiceView,
}

#[function_component(Service)]
fn service(props: &ServiceProperties) -> Html {
    html! {
        <ul>
            {
                for props.service.methods.iter().cloned().map(|method| {
                    html!{ <Method method={method} select_method={props.select_method.clone()} selected={props.selected}/> }
                })
            }
        </ul>
    }
}

impl Component for Ui {
    type Message = UiMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut endpoint = Endpoint::default();
        endpoint.tls = Some(Default::default());
        endpoint.authority = "localhost:7575".to_string();

        Self {
            new_descriptor_path: "/home/elia/code/blossom/playground/proto/playground.desc"
                .to_string(),
            repo_view: None,
            selected: None,

            tls_enabled: false,
            endpoint,

            input: String::new(),

            selected_output: 0,
            outputs: Vec::new(),

            metadata: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UiMsg::SetDescriptorPath(descriptor_path) => {
                self.new_descriptor_path = descriptor_path;
                true
            }
            UiMsg::AddProtobufDescriptor => {
                let fut = add_protobuf_descriptor(&self.new_descriptor_path);
                ctx.link().send_future(async {
                    fut.await.unwrap();
                    UiMsg::SetRepoView(get_repo_view().await.unwrap())
                });
                self.new_descriptor_path = String::new();
                false
            }
            UiMsg::SetRepoView(repo_view) => {
                self.repo_view = Some(repo_view);
                true
            }
            UiMsg::SetInput(input) => {
                self.input = input;
                true
            }
            UiMsg::ResetOutputs => {
                self.outputs = Vec::new();
                self.selected_output = 0;
                true
            }
            UiMsg::AddOutput(output) => {
                self.outputs.push(output);
                true
            }
            UiMsg::SelectMethod(serial) => {
                self.selected = Some(serial);
                true
            }

            UiMsg::SetAuthority(authority) => {
                self.endpoint.authority = authority;
                true
            }
            UiMsg::SetTLSEnable(enable) => {
                self.tls_enabled = enable;
                true
            }
            UiMsg::SetTLSNoCheck(no_check) => {
                self.endpoint.tls.as_mut().unwrap().no_check = no_check;
                true
            }
            UiMsg::SetTLSCACert(ca_cert) => {
                self.endpoint.tls.as_mut().unwrap().ca_cert = if ca_cert.is_empty() {
                    None
                } else {
                    Some(ca_cert)
                };
                true
            }

            UiMsg::SetMetadata(metadata) => {
                self.metadata = metadata;
                true
            }

            UiMsg::Prev => {
                self.selected_output -= 1;
                true
            }
            UiMsg::Next => {
                self.selected_output += 1;
                true
            }

            UiMsg::Call => {
                let input = self.input.clone();
                let selected = self.selected.unwrap();
                let mut endpoint = self.endpoint.clone();
                if !self.tls_enabled {
                    endpoint.tls = None;
                }
                let metadata_raw = self.metadata.clone();

                ctx.link().send_future(async move {
                    let mut metadata = Vec::new();
                    for line in metadata_raw.lines() {
                        if let Some((key, value)) = line.split_once(':') {
                            metadata.push((key, value));
                        }
                    }

                    UiMsg::AddOutput(unary(
                        &endpoint,
                        selected,
                        &input,
                        &metadata,
                    ).unwrap().await.unwrap())
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetDescriptorPath(e.value()))
        });

        let add_protobuf_descriptor = ctx.link().callback(|_| UiMsg::AddProtobufDescriptor);

        let send = ctx
            .link()
            .callback(|_| UiMsg::Call);

        let select_method = ctx.link().callback(|serial| {
            UiMsg::SelectMethod(serial)
        });

        let set_authority = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetAuthority(e.value()))
        });

        let set_tls_enable = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetTLSEnable(e.checked()))
        });

        let set_tls_nocheck = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetTLSNoCheck(e.checked()))
        });

        let set_tls_cacert = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetTLSCACert(e.value()))
        });

        let set_metadata = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlTextAreaElement>()
                .map(|e| UiMsg::SetMetadata(e.value()))
        });

        html! {
            <div style="display: flex; flex-direction: column; padding: 12px" id="app">
                <input type="text" value={self.endpoint.authority.clone()} oninput={set_authority} placeholder="192.168.0.1:7575"/>
                <div style="display: flex; align-items: center; flex-direction: row">
                    <input type="checkbox" name="tls_enable" oninput={set_tls_enable} checked={self.tls_enabled}/>
                    <label for="tls_enable" style="margin-right: 8px">{ "Use TLS" }</label>
                    <div class={if self.tls_enabled {classes![]} else {classes!["disabled"]}} style="flex: 1; border: 1px solid #BBBBBB; padding: 4px; margin: 4px; display: flex; flex-direction: row; align-items: center">
                        <input type="checkbox" name="tls_nocheck" oninput={set_tls_nocheck} checked={self.endpoint.tls.as_ref().unwrap().no_check}/>
                        <label for="tls_nocheck" style="margin-right: 18px">{ "Disable certificate check" }</label>

                        <input type="text" style="flex: 1" placeholder="(Optional) Path to CA certificate" oninput={set_tls_cacert} value={self.endpoint.tls.as_ref().unwrap().ca_cert.as_ref().cloned().unwrap_or_default()}/>
                    </div>
                </div>
                <div style="height: 6px"/>
                <input type="text" value={self.new_descriptor_path.clone()} {oninput}/>
                <button onclick={add_protobuf_descriptor}>{ "Add protobuf descriptor" }</button>
                <div style="height: 6px"/>
                <div style="min-height: 100px; background: #DDDDDD">
                {
                    if let Some(repo_view) = self.repo_view.as_ref() {
                        repo_view.services.iter().cloned().map(|service| {
                            html!{ <Service service={ service } select_method={select_method.clone()} selected={self.selected}/> }
                        }).collect::<Html>()
                    } else {
                        Html::default()
                    }
                }
                </div>
                <div style="height: 6px"/>
                <div style="flex: 3; display: flex; flex-direction: row; align-items: stretch">
                    <textarea value={self.input.clone()} placeholder="Write your input message here"
                        oninput={
                            ctx.link().batch_callback(|e: InputEvent| {
                                e.target_dyn_into::<HtmlTextAreaElement>()
                                    .map(|e| UiMsg::SetInput(e.value()))
                            })
                        }
                        id="input" style="flex: 1"/>
                    <div style="display: flex; flex-direction: row; align-items: stretch; flex: 1">
                        <button disabled={self.selected_output == 0} onclick={
                            ctx.link().callback(|_| UiMsg::Prev)
                        }>{"<<"}</button>
                        <textarea value={self.outputs.get(self.selected_output).cloned().unwrap_or_default()} placeholder="Get your output message here"
                            id="output" style="flex: 1" readonly={true}/>
                        <button disabled={self.outputs.len() <= 1 || self.selected_output >= self.outputs.len() - 1} onclick={
                            ctx.link().callback(|_| UiMsg::Next)
                        }>{">>"}</button>
                    </div>
                </div>
                <div style="height: 6px"/>
                <textarea placeholder="(Optional) Metadata goes here" style="flex: 2" oninput={set_metadata} value={self.metadata.clone()}/>
                <button onclick={ send }>{ "Send" }</button>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Ui>();
}
