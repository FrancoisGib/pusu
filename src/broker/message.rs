use serde::{Deserialize, Serialize};

pub struct Message<T> {
    pub id: usize,
    pub payload: T,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    ACK,
    FAILED,
}
