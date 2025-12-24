use crate::{bail, receiver::Receiver};
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    io::{ErrorKind, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{SendError, Sender},
    },
    thread::JoinHandle,
};
use thiserror::Error;

pub use pusu_producer_macro::producer;
pub mod config;

const SHARD_COUNT: usize = 64;

#[derive(Error, Debug)]
pub enum ProducerError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Binary conversion error: {0}")]
    BinaryConversionError(#[from] postcard::Error),

    #[error("No receiver available")]
    NoReceiverAvailable,

    #[error("Send error: {0}")]
    SendError(#[from] SendError<()>),
}

type Result<T> = std::result::Result<T, ProducerError>;

#[derive(PartialEq)]
pub enum ReceiverStatus {
    Available,
    Down,
}

pub struct TopicManager {
    receivers: Mutex<Vec<usize>>,
    messages_to_validate: Vec<Mutex<HashMap<usize, Vec<Vec<u8>>>>>,
}

impl TopicManager {
    pub fn new() -> Self {
        let mut shards = Vec::with_capacity(SHARD_COUNT);
        for _ in 0..SHARD_COUNT {
            shards.push(Mutex::new(HashMap::new()));
        }

        Self {
            receivers: Default::default(),
            messages_to_validate: shards,
        }
    }
}

pub struct ProducerManager<T> {
    count: AtomicUsize,
    _producer_id: usize,
    topics: HashMap<T, TopicManager>,
    receivers: Mutex<HashMap<usize, Receiver>>,
}

pub trait TopicContainer {
    fn get_topics() -> Vec<Self>
    where
        Self: Sized,
    {
        Default::default()
    }
}

impl<T: TopicContainer + Eq + Hash> ProducerManager<T> {
    pub fn new(id: usize) -> Self {
        let topics = T::get_topics()
            .into_iter()
            .map(|topic| (topic, TopicManager::new()))
            .collect();
        Self {
            count: AtomicUsize::new(0),
            _producer_id: id,
            receivers: Default::default(),
            topics,
        }
    }
}

impl<T: Serialize + Eq + Hash + Copy + Debug + Sync + Send + 'static> ProducerManager<T> {
    pub fn run(&self, rx: std::sync::mpsc::Receiver<()>) -> Result<()> {
        let listener = TcpListener::bind("localhost:8080")?;
        listener.set_nonblocking(true)?;

        loop {
            if rx.try_recv().is_ok() {
                break;
            }
            match listener.accept() {
                Ok((mut _stream, _addr)) => {
                    // let mut buf = [0u8; 1024];
                    // stream.read(&mut buf).unwrap();
                    // let (id, topic) = parse_message(&buf).unwrap();
                    // self._validate_message(topic, id);
                    continue;
                }
                Err(err) => {
                    if err.kind() != ErrorKind::WouldBlock {
                        eprintln!("Error accepting connection: {}", err);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn send<P: Serialize>(&self, topic: T, payload: &P) -> Result<()> {
        let receiver_id = self
            .get_receiver_id_by_topic(topic)
            .ok_or(ProducerError::NoReceiverAvailable)?;

        let message_id = self.generate_message_id();

        let mut lock = self.receivers.lock().unwrap();
        let receiver = lock
            .get_mut(&receiver_id)
            .ok_or(ProducerError::NoReceiverAvailable)?;

        let message_bytes = self.generate_message(message_id, topic, payload)?;

        println!("Sending to {} on topic {:?}", receiver_id, topic);

        match self.send_internal(&message_bytes, receiver) {
            Ok(_) => {
                self.add_message_to_validate(topic, message_id, message_bytes);
                Ok(())
            }
            Err(err) => {
                if matches!(err, ProducerError::IO(_)) {
                    self.mark_receiver_down(&receiver_id);
                }
                bail!(err)
            }
        }
    }

    fn send_internal(&self, message: &[u8], receiver: &mut Receiver) -> Result<()> {
        let addr = receiver.addr;
        let stream = receiver
            .stream
            .get_or_insert_with(|| TcpStream::connect(addr).unwrap());

        stream.write_all(message)?;
        Ok(())
    }

    pub fn add_receiver(&self, id: usize, addr: SocketAddr, topics: Vec<T>) -> bool {
        for topic in topics {
            self.topics
                .get(&topic)
                .unwrap()
                .receivers
                .lock()
                .unwrap()
                .push(id);
        }
        let receiver = Receiver::new(id, addr);
        self.receivers
            .lock()
            .unwrap()
            .insert(receiver.id, receiver)
            .is_none()
    }

    fn get_receiver_id_by_topic(&self, topic: T) -> Option<usize> {
        let topic_manager = if let Some(topic_manager) = self.topics.get(&topic) {
            topic_manager
        } else {
            return None;
        };

        topic_manager
            .receivers
            .lock()
            .unwrap()
            .iter()
            .find(|i| {
                let lock = self.receivers.lock().unwrap();
                let receiver = lock.get(&i).unwrap();
                receiver.status == ReceiverStatus::Available
            })
            .copied()
    }

    fn generate_message_id(&self) -> usize {
        self.count.fetch_add(1, Ordering::Relaxed)
    }

    fn generate_message<P: Serialize>(
        &self,
        message_id: usize,
        topic: T,
        payload: &P,
    ) -> Result<Vec<u8>> {
        let message_id_bytes = postcard::to_stdvec(&message_id)?;
        let topic_bytes = postcard::to_stdvec(&topic)?;
        let payload_bytes = postcard::to_stdvec(payload)?;

        let message_id_bytes_len = message_id_bytes.len() as u16;
        let topic_bytes_len = topic_bytes.len() as u16;
        let payload_bytes_len = payload_bytes.len() as u32;

        let message_id_be_len = message_id_bytes_len.to_be_bytes();
        let topic_be_len = topic_bytes_len.to_be_bytes();
        let payload_be_len = payload_bytes_len.to_be_bytes();

        let total_len =
            message_id_bytes_len as u32 + topic_bytes_len as u32 + payload_bytes_len + 2 + 2 + 4;
        let mut buf = Vec::with_capacity(total_len as usize);

        buf.extend_from_slice(&message_id_be_len);
        buf.extend_from_slice(&message_id_bytes);
        buf.extend_from_slice(&topic_be_len);
        buf.extend_from_slice(&topic_bytes);
        buf.extend_from_slice(&payload_bytes);
        buf.extend_from_slice(&payload_be_len);

        Ok(buf)
    }

    fn add_message_to_validate(&self, topic: T, message_id: usize, message: Vec<u8>) {
        let topic_manager = self.topics.get(&topic).unwrap();

        let shard = shard_index(message_id);
        let mut map = topic_manager.messages_to_validate[shard].lock().unwrap();

        println!("Message added to shard {} {:?}", shard, topic);
        map.entry(message_id).or_default().push(message);
    }

    fn _validate_message(&self, topic: T, message_id: usize) -> Option<Vec<Vec<u8>>> {
        let topic_manager = self.topics.get(&topic)?;

        println!("Validating message {} {:?}", message_id, topic);
        let shard = shard_index(message_id);
        let mut map = topic_manager.messages_to_validate[shard].lock().unwrap();

        map.remove(&message_id)
    }

    fn mark_receiver_down(&self, receiver_id: &usize) {
        let mut lock = self.receivers.lock().unwrap();
        lock.get_mut(receiver_id).unwrap().status = ReceiverStatus::Down;
    }
}

pub struct StateHandle {
    sender: Sender<()>,
    _handle: JoinHandle<()>,
}

impl StateHandle {
    pub fn new(sender: Sender<()>, handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            _handle: handle,
        }
    }

    pub fn close(self) -> Result<()> {
        self.sender.send(())?;
        self._handle.join().expect("Thread join failed");
        drop(self.sender);
        Ok(())
    }
}

#[inline]
fn shard_index(message_id: usize) -> usize {
    message_id % SHARD_COUNT
}
