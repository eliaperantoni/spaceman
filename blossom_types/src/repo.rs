use serde::{Deserialize, Serialize};

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
}

impl RepoView {
    pub fn find_method_desc(&self, target: &str) -> Option<MethodView> {
        // TODO Someday in the future, I should probably find a way to make this
        // lookup faster cause right now it takes
        // O(|services| + |methods per service|)
        let service = self
            .services
            .iter()
            .find(|&service| target.starts_with(&service.full_name));
        if let Some(service) = service {
            service
                .methods
                .iter()
                .find(|&method| method.full_name == target)
                .cloned()
        } else {
            None
        }
    }
}
