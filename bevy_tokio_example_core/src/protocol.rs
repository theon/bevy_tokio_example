use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NetworkMessage {
    Join {
        name: String,
    },
    ChatMessage {
        from: String,
        message: String,
    },
}
