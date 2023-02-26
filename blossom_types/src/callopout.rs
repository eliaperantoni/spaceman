use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CallOpOut {
    Msg(String),
    Commit,
    InvalidInput,
    InvalidOutput,
    Err(String),
}
