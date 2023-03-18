use web_sys::HtmlInputElement;
use yew::prelude::*;
use blossom_types::settings::Settings;
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
                            <div class="path-row">
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
                                    class="path-input"
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
                        text="Add"/>
                    <span class="subtitle">{ "Profiles" }</span>
                </div>
            </div>
        }
    }
}
