use crate::endpoint::Endpoint;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub endpoint: Endpoint,
    pub ordinal: i64,
}

impl Profile {
    pub fn new(ordinal: i64) -> Self {
        Profile {
            name: String::new(),
            endpoint: Endpoint::default(),
            ordinal,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub proto_paths: Vec<String>,
    pub profiles: HashMap<Uuid, Profile>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings = Self {
            proto_paths: vec![
                "/home/elia/code/blossom/playground/proto/playground.desc".to_string(),
                "/home/elia/code/proto/ono/logistics/server/ono_logistics_server.desc".to_string(),
            ],
            profiles: HashMap::new(),
        };
        settings.profiles.insert(Uuid::new_v4(), Profile {
            name: "Local 7575".to_string(),
            endpoint: Endpoint {
                authority: "localhost:7575".to_string(),
                tls: None,
            },
            ordinal: -1,
        });
        settings
    }
}
