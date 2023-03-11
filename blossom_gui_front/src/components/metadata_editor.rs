use yew::*;
use web_sys::HtmlInputElement;

use super::button::Button;
use crate::MetadataRow;

pub struct MetadataEditor {}

#[derive(PartialEq, Properties)]
pub struct MetadataEditorProps {
    pub rows: Vec<MetadataRow>,
    pub new_row: Callback<()>,
    pub update_row: Callback<(usize, MetadataRow)>,
    pub delete_row: Callback<usize>,
}

impl Component for MetadataEditor {
    type Message = ();
    type Properties = MetadataEditorProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="metadata-editor">
                {for ctx.props().rows.iter().cloned().enumerate().map(|(idx, row)| {
                    let update_row = ctx.props().update_row.clone();
                    let update_row = Callback::from(move |row: MetadataRow| {
                        update_row.emit((idx, row));
                    });

                    let delete_row = ctx.props().delete_row.clone();
                    let delete_row = Callback::from(move |_| {
                        delete_row.emit(idx);
                    });

                    html! { <RowComponent { row } { update_row } { delete_row }></RowComponent> }
                })}
                <Button
                    onclick={{
                        let new_row = ctx.props().new_row.clone();
                        Callback::from(move |_| new_row.emit(()))
                    }}
                    icon="img/plus.svg"
                    text="Add"/>
            </div>
        }
    }
}

struct RowComponent {}

#[derive(PartialEq, Properties)]
struct RowProps {
    row: MetadataRow,
    update_row: Callback<MetadataRow>,
    delete_row: Callback<()>,
}

impl Component for RowComponent {
    type Message = ();
    type Properties = RowProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="row">
                <input type="text" placeholder="Key" class="key" value={ctx.props().row.key.clone()} oninput={{
                    let (row, update_row) = (ctx.props().row.clone(), ctx.props().update_row.clone());
                    move |ev: InputEvent| {
                        let ev: HtmlInputElement = ev.target_unchecked_into();
                        update_row.emit(MetadataRow {key: ev.value(), ..row.clone()})
                    }
                }}/>
                <input type="text" placeholder="Value" class="val" value={ctx.props().row.val.clone()} oninput={{
                    let (row, update_row) = (ctx.props().row.clone(), ctx.props().update_row.clone());
                    move |ev: InputEvent| {
                        let ev: HtmlInputElement = ev.target_unchecked_into();
                        update_row.emit(MetadataRow {val: ev.value(), ..row.clone()})
                    }
                }}/>
                <img class="delete" src="img/trash-can.svg" onclick={{
                    let delete_row = ctx.props().delete_row.clone();
                    move |_| {
                        delete_row.emit(());
                    }
                }}/>
            </div>
        }
    }
}
