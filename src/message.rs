use serde::{Deserialize, Serialize};
use strum::EnumString;
use thiserror::Error;

#[derive(EnumString, Serialize, Deserialize, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum MessageStatus {
    Ack,
    Failed,
    Unknown,
}

#[derive(Debug, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum MessageType {
    Event,
    Replicate,
    Ack,
}

#[derive(Error, Debug)]
pub enum MessageStatusError {
    #[error("Topic \"{0}\" unknown")]
    UnknownTopic(String),

    #[error("{0}")]
    Failed(#[from] anyhow::Error),
}

impl From<MessageStatusError> for MessageStatus {
    fn from(value: MessageStatusError) -> Self {
        match value {
            MessageStatusError::UnknownTopic(_) => MessageStatus::Unknown,
            MessageStatusError::Failed(_) => MessageStatus::Failed,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageId {
    pub producer_id: usize,
    pub message_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T, P> {
    #[serde(rename = "type")]
    message_type: MessageType,
    id: MessageId,
    topic: T,
    payload: P,
}