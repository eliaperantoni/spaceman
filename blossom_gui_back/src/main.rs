#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{path::Path, sync::RwLock};

use tauri::State;

use blossom_core::{Conn, DynamicMessage, IntoRequest, Metadata, MethodLut, Repo, SerializeOptions};
use blossom_types::repo::Serial;

static SERIALIZE_OPTIONS: &'static SerializeOptions =
    &SerializeOptions::new().skip_default_fields(false);

fn main() {
    tauri::Builder::default()
        .manage(RwLock::new(Repo::new()))
        .manage::<RwLock<Option<MethodLut>>>(RwLock::new(None))
        .invoke_handler(tauri::generate_handler![
            get_repo_view,
            add_protobuf_descriptor,
            unary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Returns JSON encoded `RepoView`
#[tauri::command]
fn get_repo_view(
    repo: State<RwLock<Repo>>,
    lut_state: State<RwLock<Option<MethodLut>>>,
) -> Result<String, String> {
    let (repo_view, lut) = repo.read().expect("previous holder panicked").view();
    *lut_state.write().expect("previous holder panicked") = Some(lut);

    serde_json::to_string(&repo_view).map_err(|err| err.to_string())
}

#[tauri::command]
fn add_protobuf_descriptor(path: &Path, repo: State<RwLock<Repo>>) -> Result<(), String> {
    let mut repo = repo.write().expect("previous holder panicked");
    repo.add_descriptor(path).map_err(|err| err.to_string())
}

#[tauri::command]
async fn unary(
    endpoint: &str,
    serial: Serial,
    body: &str,
    metadata: Vec<(&str, &str)>,
    lut: State<'_, RwLock<Option<MethodLut>>>,
) -> Result<String, String> {
    let endpoint =
        serde_json::from_str(endpoint).map_err(|_err| "unable to parse endpoint".to_string())?;

    let method = {
        lut.read()
            .expect("previous holder panicked")
            .as_ref()
            .expect("frontend to call `get_repo_view` before making any request")
            .lookup(serial)
            .cloned()
            .ok_or_else(|| "no such method".to_string())?
    };

    let mut de = serde_json::Deserializer::from_str(body);
    let body = DynamicMessage::deserialize(method.input(), &mut de)
        .map_err(|_err| "could not parse request body".to_string())?;

    let mut req = body.into_request();

    let mut metadata_parsed = Metadata::default();
    for (key, value) in metadata {
        if key.ends_with("-bin") {
            let value = base64::decode(value).map_err(|_err| {
                "error parsing base64".to_string()
            })?;
            metadata_parsed.add_bin(key.to_string(), value).expect("key to end with -bin");
        } else {
            metadata_parsed.add_ascii(key.to_string(), value.to_string()).expect("key to not end with -bin");
        }
    }
    *req.metadata_mut() = metadata_parsed.finalize().map_err(|_err| {
        "error parsing metadata".to_string()
    })?;

    let conn =
        Conn::new(&endpoint).map_err(|_err| "could not set up connection to server".to_string())?;

    conn.unary(&method, req)
        .await
        .map_err(|_err| "error during request".to_string())
        .and_then(|res| {
            let mut se = serde_json::Serializer::pretty(vec![]);
            res.get_ref()
                .serialize_with_options(&mut se, SERIALIZE_OPTIONS)
                .map_err(|_err| "could not parse response body".to_string())?;
            Ok(String::from_utf8(se.into_inner())
                .expect("`serde_json` to never output invalid utf8"))
        })
}
