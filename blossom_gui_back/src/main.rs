#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{collections::HashMap, path::Path, sync::{RwLock, Arc, Mutex}, time::Duration};

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
            start_call,
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
    InvalidInput,
    InvalidOutput,
    Err(String),
}

#[tauri::command]
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
    let (is_client_streaming, is_server_streaming) = (method.is_client_streaming(), method.is_server_streaming());

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

    let send_outbound = {
        // Get an app handle to be able to emit events
        let app_handle = app_handle.clone();
        move |op: &CallOpOut| {
            let op_str = serde_json::to_string(op).expect("no error encoding CallOpOut");
            app_handle.emit_all(&chan_out_name, op_str).expect("no error emitting event to all windows");
        }
    };

    // Create channel that we're going to use to send messages to the request
    // router. It's either going to be just one message for non-client-streaming
    // requests or a bunch of them for clien-streaming requests.
    //
    // We are NOT going to clone the Sender which is simply moved inside the
    // closure (albeit, inside a Mutex<Option>) that handles events from the
    // frontend. When the Sender is dropped then the stream is committed and the
    // request router knows to commit the gRPC channel as well.
    // 
    // If the Receiver is dropped, it means that the request router is no longer
    // accepting incoming messages because:
    //  1) The request has just terminated
    //  2) The request has been canceled and the future associated with the
    //     request router has been dropped
    //
    // For non-client-streaming requests, the frontend is expected to send one
    // messsage withing a short span of time. The actual gRPC call happens then.
    // If no message is received within the time frame, the call is simply
    // ignored. If, after sending one message, more messages are sent or the
    // Sender is dropped, nothing happens. But be warned that you might fill the
    // buffer, in which case the application crashes.
    let (in_msg_tx, mut in_msg_rx) = tauri::async_runtime::channel::<DynamicMessage>(16);
    // We wrap the Sender in a Mutex<Option> to be able to drop it at will
    // because this makes the gRPC channel commit.
    let maybe_in_msg_tx = Mutex::new(Some(in_msg_tx));

    // When the frontend asks us to brutally cancel the request, se send a value
    // of true here which makes a tokio::select! (further up in the code)
    // complete and drop the future that runs the request router, which in turns
    // drops the gRPC client.
    //
    // The Receiver is always dropped before the Sender.
    let (cancelled_tx, mut cancelled_rx) = tokio::sync::watch::channel(false);

    let cb = {
        // We need a receiver to check if we have cancelled the stream already
        // so we don't bother to handle more inputs from the frontend.
        let cancelled_rx = cancelled_rx.clone();
        // Move inside closure
        let input_msg_type = method.input();
        // Clone closure because we'll need it again later
        let send_outbound = send_outbound.clone();

        move |ev: tauri::Event| {
            if *cancelled_rx.borrow() {
                // Stream is cancelled
                return;
            }

            let mut maybe_in_msg_tx = maybe_in_msg_tx.lock().expect("previous holder not to panic");
            let in_msg_tx = if let Some(in_msg_tx) = maybe_in_msg_tx.as_ref() {
                in_msg_tx
            } else {
                // This would mean that the stream is already committed because
                // we have already dropped the sending half
                return;
            };

            let op: CallOpIn = serde_json::from_str(ev.payload().expect("event to have a payload")).expect("no error decoding CallOpIn");
            match op {
                CallOpIn::Msg(msg_str) => {
                    let mut de = serde_json::Deserializer::from_str(&msg_str);
                    let msg = if let Ok(msg) = DynamicMessage::deserialize(input_msg_type.clone(), &mut de) {
                        msg
                    } else {
                        send_outbound(&CallOpOut::InvalidInput);
                        return;
                    };
    
                    use tokio::sync::mpsc::error::TrySendError;
                    match in_msg_tx.try_send(msg) {
                        Ok(_) => (),
                        Err(TrySendError::Closed(_)) => {
                            // Call is already terminating or cancelled so no big deal
                        },
                        Err(TrySendError::Full(_)) => {
                            panic!("message buffer is full");
                        }
                    }
                },
                CallOpIn::Commit if is_client_streaming => {
                    // Commit the stream by dropping the sending half
                    maybe_in_msg_tx.take();
                },
                CallOpIn::Cancel if is_client_streaming => {
                    // Cancel the stream, this makes a tokio::select! (further
                    // up in the code) complete and drop the future associated
                    // with the request router.
                    //
                    // Ignore previous value.
                    let _ = cancelled_tx.send_replace(true);
                },
                CallOpIn::Commit | CallOpIn::Cancel => {
                    panic!("unexpected message in non-streaming request");
                }
            };
        }
    };
    
    let event_handler = app_handle.listen_global(chan_in_name.clone(), cb);

    let main_fut = async move {
        let req = if is_client_streaming {
            let in_msg_stream = tokio_stream::wrappers::ReceiverStream::new(in_msg_rx);
            let req = in_msg_stream.into_streaming_request();
            either::Right(req)
        } else {
            // If frontend doesn't send a message within a reasonable timeframe
            // (or cancels the call), simply abort the process
            let msg = tokio::time::timeout(
                Duration::from_secs(60), in_msg_rx.recv()
            ).await.ok()??;
            let req = msg.into_request();
            either::Left(req)
        };

        // TODO Handle error in creating connection and error in making call
        //  by sending a CallOpOut::Err

        let conn = Conn::new(&endpoint).expect("no error creating connection");

        let mut res = match (req, is_server_streaming) {
            (either::Left(req), false) => {
                either::Left(conn.unary(&method, req).await.expect("no error in call"))
            }
            (either::Left(req), true) => {
                either::Right(conn.server_streaming(&method, req).await.expect("no error in call"))
            }
            (either::Right(req), false) => {
                either::Left(conn.client_streaming(&method, req).await.expect("no error in call"))
            }
            (either::Right(req), true) => {
                either::Right(conn.bidi_streaming(&method, req).await.expect("no error in call"))
            }
        };

        match res {
            either::Left(res) => {
                if let Ok(msg_str) = serde_json::to_string(res.get_ref()) {
                    send_outbound(&CallOpOut::Msg(msg_str));
                } else {
                    send_outbound(&CallOpOut::InvalidOutput);
                }
            }
            either::Right(mut res) => {
                loop {
                    match res.get_mut().next().await {
                        Some(Ok(msg)) => {
                            if let Ok(msg_str) = serde_json::to_string(&msg) {
                                send_outbound(&CallOpOut::Msg(msg_str));
                            } else {
                                send_outbound(&CallOpOut::InvalidOutput);
                            }
                        },
                        Some(Err(err)) => {
                            send_outbound(&CallOpOut::Err(err.to_string()));
                            break;
                        },
                        None => {
                            // No more messages
                            send_outbound(&CallOpOut::Commit);
                            break;
                        }
                    }
                }
            }
        }

        Some(())
    };

    tauri::async_runtime::spawn(async move {
        tokio::select! {
            _ = main_fut => (),
            // This is never Err because cancelled_tx is only ever dropped by
            // unregistering the event handler, and this happens only when this
            // tokio::select! completes and all leftover futures are dropped.
            _ = cancelled_rx.changed() (),
        };
        // main_fut is already dropped by now so there's no risk to trigger any
        // specific behavior by dropping the closure and all Sender/Receiver
        // that it might own.
        app_handle.unlisten(event_handler);
    });

    Ok(call_id)
}
