use std::future::Future;

use js_sys::{JsString, Object, Reflect, Array};
use wasm_bindgen::JsValue;

use crate::glue::invoke;
use crate::call::{start_call, message, commit, cancel, listen, self};

use blossom_types::{
    endpoint::Endpoint,
    repo::{RepoView, Serial},
};

use futures::stream::StreamExt;
use futures::sink::SinkExt;

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
