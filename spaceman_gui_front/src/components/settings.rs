use web_sys::HtmlInputElement;
use yew::prelude::*;
use spaceman_types::{settings::{Settings, Profile}, endpoint::Endpoint};
use uuid::Uuid;

use crate::components::button::Button;

pub struct SettingsEditor {}

#[derive(PartialEq, Properties)]
pub struct SettingsProps {
    pub settings: Settings,
    pub set_settings: Callback<Settings>,
    pub leave_settings: Callback<()>,
}

impl Component for SettingsEditor {
    type Message = ();
    type Properties = SettingsProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="settings">
                <img onclick={ ctx.props().leave_settings.clone().reform(|_| ()) } class="leave-settings" src="img/arrow-left.svg"/>
                <div class="content">
                    <span class="title">{ "Settings" }</span>
                    <span class="subtitle">{ "Protos" }</span>
                    {
                        ctx.props().settings.proto_paths.iter().enumerate().map(|(idx, path)| html!{
                            <div class="row">
                                <input 
                                    value={ path.clone() }
                                    oninput={
                                        let settings = ctx.props().settings.clone();
                                        ctx.props().set_settings.clone().reform(move |ev: InputEvent| {
                                            let path = ev.target_unchecked_into::<HtmlInputElement>().value();
                                            let mut settings = settings.clone();
                                            settings.proto_paths[idx] = path;
                                            settings
                                        })
                                    }
                                    placeholder="Path to the proto descriptor"
                                    class="input"
                                    type="text"/>
                                <img class="delete" src="img/trash-can.svg" onclick={{
                                    let settings = ctx.props().settings.clone();
                                    ctx.props().set_settings.clone().reform(move |_| {
                                        let mut settings = settings.clone();
                                        settings.proto_paths.remove(idx);
                                        settings
                                    })
                                }}/>
                            </div>
                        }).collect::<Html>()
                    }
                    <Button
                        onclick={ 
                            let settings = ctx.props().settings.clone();
                            ctx.props().set_settings.clone().reform(move |_| {
                                let mut settings = settings.clone();
                                settings.proto_paths.push(String::new());
                                settings
                            })
                        }
                        icon="img/plus.svg"
                        text="Add proto"/>

                    <span class="subtitle">{ "Profiles" }</span>
                    {{
                        let mut profiles = ctx.props().settings.profiles.iter().map(|(id, profile)| {
                            (id.clone(), profile.clone())
                        }).collect::<Vec<_>>();
                        profiles.sort_by_key(|(_, profile)| profile.ordinal);
                        profiles.into_iter().enumerate().map(|(index, (id, profile))| {
                            html!{<>
                                <div class="row">
                                    <input 
                                        value={ profile.name.clone() }
                                        oninput={
                                            let id = id.clone();
                                            let settings = ctx.props().settings.clone();
                                            ctx.props().set_settings.clone().reform(move |ev: InputEvent| {
                                                let name = ev.target_unchecked_into::<HtmlInputElement>().value();
                                                let mut settings = settings.clone();
                                                settings.profiles.get_mut(&id).unwrap().name = name;
                                                settings
                                            })
                                        }
                                        placeholder="Profile name"
                                        class="input"
                                        type="text"/>
                                    <input 
                                        value={ profile.endpoint.authority.clone() }
                                        oninput={
                                            let id = id.clone();
                                            let settings = ctx.props().settings.clone();
                                            ctx.props().set_settings.clone().reform(move |ev: InputEvent| {
                                                let authority = ev.target_unchecked_into::<HtmlInputElement>().value();
                                                let mut settings = settings.clone();
                                                settings.profiles.get_mut(&id).unwrap().endpoint.authority = authority;
                                                settings
                                            })
                                        }
                                        style="flex: 2"
                                        placeholder="Authority"
                                        class="input"
                                        type="text"/>
                                    <img class="delete" src="img/trash-can.svg" onclick={{
                                        let id = id.clone();
                                        let settings = ctx.props().settings.clone();
                                        ctx.props().set_settings.clone().reform(move |_| {
                                            let mut settings = settings.clone();
                                            settings.profiles.remove(&id);
                                            settings
                                        })
                                    }}/>
                                </div>
                                <div class="row">
                                    <input
                                        checked={ profile.endpoint.tls.is_some() }
                                        onclick={ 
                                            let id = id.clone();
                                            let settings = ctx.props().settings.clone();
                                            ctx.props().set_settings.clone().reform(move |ev: MouseEvent| {
                                                let use_tls = ev.target_unchecked_into::<HtmlInputElement>().checked();
                                                let mut settings = settings.clone();
                                                settings.profiles.get_mut(&id).unwrap().endpoint.tls = if use_tls {
                                                    Some(Default::default())
                                                } else {
                                                    None
                                                };
                                                settings
                                            })
                                        }
                                        class="input"
                                        type="checkbox"/>
                                    <span>{ "Use TLS" }</span>

                                    if let Some(tls) = &profile.endpoint.tls {
                                        <input
                                            checked={ tls.no_check }
                                            onclick={
                                                let id = id.clone();
                                                let settings = ctx.props().settings.clone();
                                                ctx.props().set_settings.clone().reform(move |ev: MouseEvent| {
                                                    let no_check = ev.target_unchecked_into::<HtmlInputElement>().checked();
                                                    let mut settings = settings.clone();
                                                    settings.profiles.get_mut(&id).unwrap().endpoint.tls.as_mut().unwrap().no_check = no_check;
                                                    settings
                                                })
                                            }
                                            class="input"
                                            type="checkbox"/>
                                        <span>{ "Skip certificate check" }</span>

                                        <input
                                            value={ tls.ca_cert.clone().unwrap_or_else(|| String::new()) }
                                            oninput={
                                                let id = id.clone();
                                                let settings = ctx.props().settings.clone();
                                                ctx.props().set_settings.clone().reform(move |ev: InputEvent| {
                                                    let ca_cert = ev.target_unchecked_into::<HtmlInputElement>().value();
                                                    let mut settings = settings.clone();
                                                    settings.profiles.get_mut(&id).unwrap().endpoint.tls.as_mut().unwrap().ca_cert = if !ca_cert.is_empty() {
                                                        Some(ca_cert)
                                                    } else {
                                                        None
                                                    };
                                                    settings
                                                })
                                            }
                                            placeholder="Path to CA cert"
                                            class="input"
                                            type="text"/>
                                    } else {
                                        <div class="ghost"></div>
                                    }
                                </div>
                                if index < ctx.props().settings.profiles.len() - 1 {
                                    <div class="profile-spacer"></div>
                                }
                            </>}
                        }).collect::<Html>()
                    }}
                    <Button
                        onclick={ 
                            let settings = ctx.props().settings.clone();
                            ctx.props().set_settings.clone().reform(move |_| {
                                let mut settings = settings.clone();
                                settings.profiles.insert(Uuid::new_v4(), Profile::new(
                                    1 + settings.profiles.iter().map(|(_, profile)| profile.ordinal).max().unwrap_or_else(|| 0)
                                ));
                                settings
                            })
                        }
                        icon="img/plus.svg"
                        text="Add profile"/>
                </div>
            </div>
        }
    }
}
