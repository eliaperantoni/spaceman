#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{collections::HashMap, path::Path, sync::{RwLock, Arc, Mutex}};

use tauri::{Manager, State};
use tokio_stream::StreamExt;
use blossom_core::{Conn, DynamicMessage, IntoRequest, IntoStreamingRequest, Metadata, MethodLut, Repo, SerializeOptions};
use blossom_types::repo::Serial;

static SERIALIZE_OPTIONS: &'static SerializeOptions =
    &SerializeOptions::new().skip_default_fields(false);

fn main() {
    tauri::Builder::default()
        .manage(RwLock::new(Repo::new()))
        .manage::<RwLock<Option<MethodLut>>>(RwLock::new(None))
        .setup(|app| {
            Ok(())
        })
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

type CallId = usize;

#[derive(Debug, PartialEq)]
struct Call {
    id: CallId,
}

impl Call {
    fn new(id: CallId) -> Self {
        Call {id}
    }

    fn input_chan(&self) -> String {
        return format!("i-{}", self.id)
    }

    fn output_chan(&self) -> String {
        return format!("o-{}", self.id)
    }
}

#[derive(Debug)]
struct CallsManager {
    calls: HashMap<CallId, Call>,
    next_call_id: CallId,
}

impl CallsManager {
    fn new() -> Self {
        Self {
            calls: Default::default(),
            next_call_id: 1,
        }
    }

    fn start_request(&mut self) -> CallId {
        let id = self.next_call_id;
        self.next_call_id += 1;
        self.calls.insert(id, Call {id});
        id
    }

    fn stop_request(&mut self, call_id: CallId) {
        self.calls.remove(&call_id);
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum CallOpIn {
    Msg(String),
    Commit,
    Cancel,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum CallOpOut {
    Msg(String),
    Commit,
    Err(String),
}

fn start_call(
    endpoint_encoded: &str,
    method_serial: Serial,
    metadata: Vec<(&str, &str)>,
    lut: State<'_, RwLock<Option<MethodLut>>>,
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    let call_id = 1;
    let chan_in_name = format!("i-{}", call_id);
    let chan_out_name = format!("o-{}", call_id);

    let endpoint =
        serde_json::from_str(endpoint_encoded).map_err(|_err| "unable to parse endpoint".to_string())?;

    let method = {
        lut.read()
            .expect("previous holder panicked")
            .as_ref()
            .expect("frontend to call `get_repo_view` before making any request")
            .lookup(method_serial)
            .cloned()
            .ok_or_else(|| "no such method".to_string())?
    };

    let metadata = {
        let mut tmp = Metadata::default();
        for (key, value) in metadata {
            if key.ends_with("-bin") {
                let value = base64::decode(value).map_err(|_err| {
                    "error parsing base64".to_string()
                })?;
                tmp.add_bin(key.to_string(), value).expect("key to end with -bin");
            } else {
                tmp.add_ascii(key.to_string(), value.to_string()).expect("key to not end with -bin");
            }
        }
        tmp.finalize().map_err(|_err| {
            "error parsing metadata".to_string()
        })?
    };

    let (in_msg_tx, in_msg_rx) = tauri::async_runtime::channel::<DynamicMessage>(8);

    let event_handler = Arc::new(Mutex::new(None));

    let (cancel_tx, mut cancel_rx) = tokio::sync::mpsc::unbounded_channel();

    let input_msg_type = method.input();
    let cb = {
        // Get an app handle to be able to unlisten from further events
        let app_handle = app_handle.clone();
        // Clone the Rc containing the event handler so we can unregister it
        let event_handler = event_handler.clone();

        move |ev: tauri::Event| {
            let op: CallOpIn = serde_json::from_str(ev.payload().expect("event to have a payload")).expect("no error decoding CallOpIn");
            match op {
                CallOpIn::Msg(msg_str) => {
                    let mut de = serde_json::Deserializer::from_str(&msg_str);
                    let msg = DynamicMessage::deserialize(input_msg_type.clone(), &mut de).expect("no error decoding DynamicMessage");
    
                    let _ = in_msg_tx.blocking_send(msg);
                },
                op @ (CallOpIn::Commit | CallOpIn::Cancel) => {
                    if let Some(event_handler) = event_handler.lock().expect("previous owner not to panic").take() {
                        app_handle.unlisten(event_handler);
                    }
                    if matches!(op, CallOpIn::Cancel) {
                        // Any error would indicate that the call is already terminated so we can safely ignore
                        let _ = cancel_tx.send(());
                    }
                }
            };
        }
    };
    event_handler.lock().expect("previous holder not to panic").insert(
        app_handle.listen_global(chan_in_name.clone(), cb)
    );

    let send_outbound = {
        // Get an app handle to be able to emit events
        let app_handle = app_handle.clone();
        move |op: &CallOpOut| {
            let op_str = serde_json::to_string(op).expect("no error encoding CallOpOut");
            app_handle.emit_all(&chan_out_name, op_str).expect("no error emitting event to all windows");
        }
    };

    let (is_client_streaming, is_server_streaming) = (method.is_client_streaming(), method.is_server_streaming());
    let main_fut = tauri::async_runtime::spawn({
        async move {
            match (is_client_streaming, is_server_streaming) {
                (true, true) => {
                    let in_msg_stream = tokio_stream::wrappers::ReceiverStream::new(in_msg_rx);
                    let req = in_msg_stream.into_streaming_request();
                    
                    let conn = Conn::new(&endpoint).expect("no error creating connection");
                    let mut response = conn.bidi_streaming(&method, req).await.expect("no error starting call");
                    while let Some(maybe_msg) = response.get_mut().next().await {
                        match maybe_msg {
                            Ok(msg) => {
                                let msg_str = serde_json::to_string(&msg).expect("no error encoding DynamicMessage");
                                send_outbound(&CallOpOut::Msg(msg_str));
                            },
                            Err(err) => {
                                send_outbound(&CallOpOut::Err(err.to_string()));
                                return;
                            }
                        }
                    }
                    send_outbound(&CallOpOut::Commit);
                },
                _ => todo!(),
            };
        }
    });

    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = main_fut => {},
            _ = cancel_rx.recv() => {}
        };
        if let Some(event_handler) = event_handler.lock().expect("previous owner not to panic").take() {
            app_handle.unlisten(event_handler);
        }
    });

    Ok(call_id)
}
