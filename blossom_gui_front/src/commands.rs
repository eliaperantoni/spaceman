use std::future::Future;

use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use serde_json::to_string_pretty;

use crate::glue::invoke;

use blossom_types::{repo::RepoView, settings::Settings};

pub(crate) async fn get_repo_view() -> Result<RepoView, String> {
    invoke("get_repo_view", JsValue::NULL)
        .await
        .ok()
        .and_then(|ok| {
            serde_json::from_str(&ok.as_string().expect("backend to return a string here")).ok()
        })
        .ok_or_else(|| "error reading repository".to_string())
}

pub(crate) fn add_protobuf_descriptor(path: &str) -> impl Future<Output = Result<(), String>> {
    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("path"),
        &js_sys::JsString::from(path),
    )
    .unwrap();
    async {
        invoke("add_protobuf_descriptor", o.into())
            .await
            .map(|_| ())
            .map_err(|_err| "error adding protobuf descriptor".to_string())
    }
}

pub(crate) async fn reset_repo() -> Result<(), String> {
    invoke("reset_repo", JsValue::NULL)
        .await
        .map(|_| ())
        .map_err(|_err| "error resetting loaded protos".to_string())
}

pub(crate) async fn get_empty_input_message(method_full_name: &str) -> Result<String, String> {
    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("methodFullName"),
        &wasm_bindgen::JsValue::from(method_full_name),
    )
    .unwrap();

    invoke("get_empty_input_message", o.into())
        .await
        .ok()
        .and_then(|ok| ok.as_string())
        .ok_or_else(|| "error getting empty input message".to_string())
}

pub(crate) async fn save_settings(settings: &Settings) -> Result<(), String> {
    let content = to_string_pretty(settings).map_err(|err| err.to_string())?;

    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("content"),
        &wasm_bindgen::JsValue::from(&content),
    )
    .unwrap();

    invoke("save_settings", o.into())
        .await
        .map(|_| ())
        .map_err(|err| err.as_string().unwrap())
}

pub(crate) async fn load_settings() -> Result<Option<Settings>, String> {
    let o = Object::new();

    invoke("load_settings", o.into())
        .await
        .map_err(|err| err.as_string().unwrap_or_else(|| "error reading settings".to_string()))
        .and_then(|content| {
            let content = content.as_string().unwrap_or_else(|| String::new());
            if content.is_empty() {
                return Ok(None);
            }

            let settings = serde_json::from_str::<Settings>(&content);
            settings.map(|settings| Some(settings)).map_err(|err| err.to_string())
        })
}
