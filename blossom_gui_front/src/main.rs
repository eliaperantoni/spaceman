use std::future::Future;
use std::io::SeekFrom::End;

use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use blossom_types::repo::{MethodView, RepoView, Serial, ServiceView};
use blossom_types::endpoint::{Endpoint};

use crate::command::*;

mod command;

enum UiMsg {
    SetDescriptorPath(String),
    AddProtobufDescriptor,
    SetRepoView(RepoView),
    SetInput(String),
    SetOutput(String),
    SelectMethod(Serial),
    SetAuthority(String),
    Call
}

struct Ui {
    new_descriptor_path: String,
    repo_view: Option<RepoView>,
    selected: Option<Serial>,

    authority: String,

    input: String,
    output: String,
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
        Self {
            new_descriptor_path: "/home/elia/code/blossom/playground/proto/playground.desc"
                .to_string(),
            repo_view: None,
            selected: None,
            authority: String::new(),
            input: String::new(),
            output: String::new(),
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
            UiMsg::SetOutput(output) => {
                self.output = output;
                true
            }
            UiMsg::SelectMethod(serial) => {
                self.selected = Some(serial);
                true
            }
            UiMsg::SetAuthority(authority) => {
                self.authority = authority;
                true
            }
            UiMsg::Call => {
                let input = self.input.clone();
                let selected = self.selected.unwrap();
                let authority = self.authority.clone();

                ctx.link().send_future(async move {
                    UiMsg::SetOutput(unary(
                        &Endpoint{
                            authority,
                            tls: None,
                        },
                        selected,
                        &input,
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

        html! {
            <div style="display: flex; flex-direction: column" id="app">
                <input type="text" value={self.new_descriptor_path.clone()} {oninput}/>
                <button onclick={add_protobuf_descriptor}>{ "Add protobuf descriptor" }</button>
                <div style="height: 6px"/>
                <input type="text" value={self.authority.clone()} oninput={set_authority} placeholder="192.168.0.1:7575"/>
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
                <div style="flex: 1; display: flex; flex-direction: row; align-items: stretch">
                    <textarea value={self.input.clone()} placeholder="Write your input message here"
                        oninput={
                            ctx.link().batch_callback(|e: InputEvent| {
                                e.target_dyn_into::<HtmlTextAreaElement>()
                                    .map(|e| UiMsg::SetInput(e.value()))
                            })
                        }
                        id="input" style="flex: 1"/>
                    <textarea value={self.output.clone()} placeholder="Get your output message here"
                        id="output" style="flex: 1" readonly={true}/>
                </div>
                <button onclick={ send }>{ "Send" }</button>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Ui>();
}
