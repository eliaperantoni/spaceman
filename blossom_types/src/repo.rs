use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoView {
    pub services: Vec<ServiceView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceView {
    pub full_name: String,
    pub parent_file: String,
    pub methods: Vec<MethodView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MethodView {
    pub name: String,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}
