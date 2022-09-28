use std::future::Future;

use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew::virtual_dom::AttrValue;

use crate::command::{add_protobuf_descriptor, get_repo_tree};

mod command;

enum UiMsg {
    SetDescriptorPath(String),
    AddProtobufDescriptor,
    SetServices(Vec<ServiceProperties>),
}

struct Ui {
    new_descriptor_path: String,
    services: Vec<ServiceProperties>,
}

#[derive(PartialEq, Clone, Properties)]
struct MethodProperties {
    name: AttrValue,
}

#[function_component(Method)]
fn method(props: &MethodProperties) -> Html {
    html! {
        <li>{ &props.name }</li>
    }
}

#[derive(PartialEq, Clone, Properties)]
struct ServiceProperties {
    methods: Vec<MethodProperties>,
}

#[function_component(Service)]
fn service(props: &ServiceProperties) -> Html {
    html!{
        <ul>
            {
                props.methods.iter().map(|props| {
                    html!{ <Method ..props.clone()/> }
                }).collect::<Html>()
            }
        </ul>
    }
}

impl Component for Ui {
    type Message = UiMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            new_descriptor_path: "/home/elia/code/blossom/playground/proto/playground.desc".to_string(),
            services: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UiMsg::SetDescriptorPath(descriptor_path) => {
                self.new_descriptor_path = descriptor_path;
                true
            },
            UiMsg::AddProtobufDescriptor => {
                let fut = add_protobuf_descriptor(&self.new_descriptor_path);
                ctx.link().send_future(async {
                    fut.await.unwrap();

                    let repo_tree = get_repo_tree().await.unwrap();
                    UiMsg::SetServices(
                        repo_tree.services.into_iter().map(|service|
                            ServiceProperties {
                                methods: service.methods.into_iter().map(|method| {
                                    MethodProperties {
                                        name: AttrValue::from(method.name),
                                    }
                                }).collect()
                            }
                        ).collect()
                    )
                });
                self.new_descriptor_path = String::new();
                false
            },
            UiMsg::SetServices(services) => {
                self.services = services;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().batch_callback(|e: InputEvent| {
            e.target_dyn_into::<HtmlInputElement>()
                .map(|e| UiMsg::SetDescriptorPath(e.value()))
        });

        let add_protobuf_descriptor = ctx.link().callback(|_| UiMsg::AddProtobufDescriptor);

        html! {
            <div>
                <input type="text" value={self.new_descriptor_path.clone()} {oninput} style="width: 500px"/>
                <button onclick={add_protobuf_descriptor}>{ "Add protobuf descriptor" }</button>
                <div style="height: 10px"/>
                {
                    self.services.iter().map(|service| {
                        html! {
                            <Service ..service.clone()/>
                        }
                    }).collect::<Html>()
                }
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Ui>();
}
