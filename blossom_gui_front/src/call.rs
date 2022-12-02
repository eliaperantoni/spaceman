use std::future::Future;

use futures::Stream;
use js_sys::{Array, Function, JsString, Object, Reflect};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

use crate::glue;

use blossom_types::{endpoint::Endpoint, repo::Serial};

pub(crate) async fn start_call(
    call_id: i32,
    endpoint: &Endpoint,
    method_serial: Serial,
    metadata: &[(&str, &str)],
) -> Result<(), String> {
    let endpoint =
        serde_json::to_string(endpoint).map_err(|_err| "error serializing endpoint".to_string())?;

    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("callId"),
        &js_sys::Number::from(call_id),
    )
    .unwrap();
    Reflect::set(
        &o,
        &js_sys::JsString::from("endpointEncoded"),
        &js_sys::JsString::from(endpoint.as_ref()),
    )
    .unwrap();
    Reflect::set(
        &o,
        &js_sys::JsString::from("methodSerial"),
        &wasm_bindgen::JsValue::from(method_serial),
    )
    .unwrap();
    let metadata_vec = Array::new();
    for &(key, value) in metadata {
        let mut pair = Array::new_with_length(2);
        pair.set(0, js_sys::JsString::from(key).into());
        pair.set(1, js_sys::JsString::from(value).into());
        metadata_vec.push(&pair);
    }
    Reflect::set(&o, &js_sys::JsString::from("metadata"), &metadata_vec).unwrap();

    glue::invoke("start_call", o.into())
        .await
        .map(|_res| ())
        .map_err(|_err| "error making unary request".to_string())
}

pub(crate) fn message(call_id: i32, body: &str) {
    let o = Object::new();
    Reflect::set(
        &o,
        &js_sys::JsString::from("Msg"),
        &js_sys::JsString::from(body),
    )
    .unwrap();

    glue::emit(&format!("o-{}", call_id), o.into());
}

pub(crate) fn commit(call_id: i32) {
    glue::emit(
        &format!("i-{}", call_id),
        // The string quoting is because emit, from tauri, does seem to skip the
        // JSON encoding when directly passed a string
        js_sys::JsString::from("\"Commit\"").into(),
    );
}

pub(crate) fn cancel(call_id: i32) {
    glue::emit(
        &format!("i-{}", call_id),
        // The string quoting is because emit, from tauri, does seem to skip the
        // JSON encoding when directly passed a string
        js_sys::JsString::from("\"Cancel\"").into(),
    );
}

pub(crate) struct Listener {
    clo: Closure<dyn FnMut(JsValue)>,
    unlisten: Function,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.unlisten.call0(&js_sys::global());
    }
}

pub(crate) async fn listen(
    call_id: i32,
    f: Box<dyn FnMut(JsValue) + 'static>,
) -> Listener {
    let chan_name = format!("o-{}", call_id);
    let clo = Closure::wrap(f);

    let unlisten = glue::listen(&chan_name, &clo).await;
    let unlisten = unlisten.unchecked_into::<Function>();

    Listener { clo, unlisten }
}
