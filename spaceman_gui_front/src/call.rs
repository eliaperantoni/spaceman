use std::future::Future;

use futures::Stream;
use js_sys::{Array, Function, JsString, Object, Reflect};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

use crate::glue;
use crate::MetadataRow;

use spaceman_types::endpoint::Endpoint;
use spaceman_types::callopout::CallOpOut;

pub(crate) async fn start_call(
    call_id: i32,
    endpoint: &Endpoint,
    method_full_name: &str,
    metadata: &[MetadataRow],
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
        &js_sys::JsString::from("methodFullName"),
        &wasm_bindgen::JsValue::from(method_full_name),
    )
    .unwrap();
    let metadata_vec = Array::new();
    for row in metadata {
        let mut pair = Array::new_with_length(2);
        pair.set(0, js_sys::JsString::from(row.key.as_str()).into());
        pair.set(1, js_sys::JsString::from(row.val.as_str()).into());
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

    glue::emit(&format!("i-{}", call_id), o.into());
}

pub(crate) fn commit(call_id: i32) {
    glue::emit(
        &format!("i-{}", call_id),
        // The string quoting is because emit, from tauri, does seem to skip the
        // JSON encoding when directly passed a string
        js_sys::JsString::from("Commit").into(),
    );
}

pub(crate) fn cancel(call_id: i32) {
    glue::emit(
        &format!("i-{}", call_id),
        // The string quoting is because emit, from tauri, does seem to skip the
        // JSON encoding when directly passed a string
        js_sys::JsString::from("Cancel").into(),
    );
}

pub(crate) struct Listener {
    clo: Closure<dyn FnMut(JsValue)>,
    unlisten: Function,
}

impl Drop for Listener {
    fn drop(&mut self) {
        _ = self.unlisten.call0(&js_sys::global()).expect("no error unregistering handler");
    }
}

pub(crate) async fn listen(
    call_id: i32,
    mut f: Box<dyn FnMut(CallOpOut) + 'static>,
) -> Listener {
    let chan_name = format!("o-{}", call_id);
    let clo = Closure::new(move |js_value| {
        let s = Reflect::get(&js_value, &JsString::from("payload")).expect("event to have a payload").as_string().expect("payload to be a string");
        let call_op_out = serde_json::from_str(&s).expect("payload to be deserializable");

        f(call_op_out)
    });

    let unlisten = glue::listen(&chan_name, &clo).await;
    let unlisten = unlisten.unchecked_into::<Function>();

    Listener { clo, unlisten }
}
