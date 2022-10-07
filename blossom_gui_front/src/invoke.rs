use wasm_bindgen::prelude::*;
use js_sys::JsString;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsString>;
}
