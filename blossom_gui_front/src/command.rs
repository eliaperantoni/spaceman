use std::future::Future;
use js_sys::{JsString, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use blossom_types::repo::RepoView;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsString>;
}

pub(crate) fn add_protobuf_descriptor(path: &str) -> impl Future<Output=Result<(), String>> {
    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("path"),
        &js_sys::JsString::from(path)
    ).unwrap();
    async {
        invoke("add_protobuf_descriptor", o.into())
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }
}

pub(crate) fn get_repo_tree() -> impl Future<Output=Result<RepoView, String>> {
    async {
        invoke("get_repo_tree", JsValue::NULL)
            .await
            .map(|ok| {
                // TODO Can this be better
                serde_json::from_str(&ok.unchecked_into::<JsString>().as_string().unwrap()).unwrap()
            })
            .map_err(|err| err.into())
    }
}
