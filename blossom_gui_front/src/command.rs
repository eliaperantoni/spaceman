use std::future::Future;

use js_sys::{JsString, Object, Reflect};
use wasm_bindgen::prelude::*;

use blossom_types::{
    endpoint::Endpoint,
    repo::{RepoView, Serial},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsString>;
}

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

pub(crate) fn unary(
    endpoint: &Endpoint,
    serial: Serial,
    body: &str,
) -> Result<impl Future<Output = Result<String, String>>, String> {
    let endpoint =
        serde_json::to_string(endpoint).map_err(|_err| "error serializing endpoint".to_string())?;

    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("endpoint"),
        &js_sys::JsString::from(endpoint.as_ref()),
    )
    .unwrap();
    Reflect::set(
        &o,
        &js_sys::JsString::from("serial"),
        &wasm_bindgen::JsValue::from(serial),
    )
    .unwrap();
    Reflect::set(
        &o,
        &js_sys::JsString::from("body"),
        &js_sys::JsString::from(body),
    )
    .unwrap();

    Ok(async {
        invoke("unary", o.into())
            .await
            .map(|res| res.as_string().expect("backend to return a string here"))
            .map_err(|_err| "error making unary request".to_string())
    })
}
