use wasm_bindgen::prelude::*;
use js_sys::{JsString, Function};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsString>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub fn emit(chan: &str, payload: JsValue);

    // The returned valhe is the function that unregisters the handler
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub async fn listen(chan: &str, callback: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}
