use crate::endpoint::Endpoint;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub proto_paths: Vec<String>,
    pub profiles: HashMap<Uuid, Profile>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            proto_paths: Vec::new(),
            profiles: HashMap::new(),
        }
    }
}
