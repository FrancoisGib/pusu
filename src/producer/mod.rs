use postcard;
use std::{io::Write, marker::PhantomData, net::TcpStream};

use anyhow::{Result, bail};
use serde::Serialize;

pub use pusu_producer_macro::producer;

#[derive(PartialEq, Clone, Copy)]
pub enum BrokerStatus {
    AVAILABLE,
    FAILED,
}

pub struct Receiver<T> {
    id: usize,
    addr: String,
    status: BrokerStatus,
    _phantom: PhantomData<T>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            addr: self.addr.clone(),
            status: self.status,
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize> Receiver<T> {
    pub fn new(id: usize, addr: &str) -> Self {
        Self {
            id,
            addr: addr.to_string(),
            status: BrokerStatus::AVAILABLE,
            _phantom: PhantomData,
        }
    }

    pub fn send(&self, topic: &str, payload: &T) -> Result<()> {
        let mut stream = TcpStream::connect(&self.addr)?;

        let topic_bytes = topic.as_bytes();
        let payload_bytes = postcard::to_stdvec(payload)?;

        let topic_len = (topic_bytes.len() as u16).to_be_bytes();
        let payload_len = (payload_bytes.len() as u32).to_be_bytes();

        stream.write_all(&topic_len)?;
        stream.write_all(topic_bytes)?;
        stream.write_all(&payload_len)?;
        stream.write_all(&payload_bytes)?;
        Ok(())
    }
}

pub struct Receivers<T> {
    i: usize,
    receivers: Vec<Receiver<T>>,
}

// Cannot derive default because of macros, otherwise all T should implement Default
impl<T> Default for Receivers<T> {
    fn default() -> Self {
        Self {
            i: Default::default(),
            receivers: Default::default(),
        }
    }
}

impl<T> Receivers<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

pub trait ReceiverDispatch<T> {
    fn add_receiver(&mut self, topic: T, id: usize, addr: &str);
}

impl<T: Serialize> Receivers<T> {
    pub fn send(&mut self, topic: &str, payload: &T) -> Result<()> {
        let len = self.receivers.len();
        if len == 0 {
            bail!("No broker available")
        }

        let mut i = self.i;

        loop {
            i += 1 % len;
            if let Some(broker) = self.receivers.get(i)
                && broker.status == BrokerStatus::AVAILABLE
            {
                broker.send(topic, payload)?;
                self.i = i;
                break;
            } else if i == self.i {
                bail!("No broker available")
            }
        }
        Ok(())
    }

    pub fn add_receiver(&mut self, id: usize, addr: &str) {
        self.receivers.push(Receiver::new(id, addr));
    }

    pub fn remove_receiver(&mut self, id: usize) {
        self.receivers.retain(|b| b.id == id);
    }
}
