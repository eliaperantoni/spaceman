use crate::endpoint::Endpoint;

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub proto_paths: Vec<String>,
    pub profiles: Vec<(String, Endpoint)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            proto_paths: vec![
                "/home/elia/code/blossom/playground/proto/playground.desc".to_string(),
                "/home/elia/code/proto/ono/logistics/server/ono_logistics_server.desc".to_string(),
            ],
            profiles: vec![
                ("Local 7575".to_string(), Endpoint {
                    authority: "localhost:7575".to_string(),
                    tls: None,
                })
            ],
        }
    }
}
