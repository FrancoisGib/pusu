use std::net::{SocketAddr, TcpStream};

use crate::producer::ReceiverStatus;

pub struct Receiver {
    pub id: usize,
    pub addr: SocketAddr,
    pub status: ReceiverStatus,
    pub stream: Option<TcpStream>,
}

impl Receiver {
    pub fn new(id: usize, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            status: ReceiverStatus::Available,
            stream: None,
        }
    }
}
