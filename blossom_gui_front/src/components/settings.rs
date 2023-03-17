use yew::prelude::*;
use crate::Settings;

pub struct SettingsEditor {}

#[derive(PartialEq, Properties)]
pub struct SettingsProps {
    pub settings: Settings,
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
                    <span class="subtitle">{ "Profiles" }</span>
                </div>
            </div>
        }
    }
}
