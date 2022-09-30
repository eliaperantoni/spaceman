use serde::{Deserialize, Serialize};

pub type Serial = usize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RepoView {
    pub services: Vec<ServiceView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceView {
    pub name: String,
    pub full_name: String,
    pub parent_file: String,
    pub methods: Vec<MethodView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodView {
    pub name: String,
    pub full_name: String,
    pub input_msg_name: String,
    pub output_msg_name: String,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,

    // Used for quick lookup
    pub serial: Serial,
}
