use std::{
    io::Read, net::{SocketAddr, TcpListener, TcpStream}, str::FromStr, sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Sender, channel},
    }, thread::{self, JoinHandle}
};

pub use pusu_consumer_macro::consumer;
use signal_hook::{consts::SIGINT, iterator::Signals};
use thiserror::Error;

use crate::{
    bail,
    message::{MessageStatus, MessageStatusError},
};

#[derive(Error, Debug)]
pub enum ConsumerError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Invalid UTF-8 payload: {0}")]
    InvalidUTF8Error(#[from] std::str::Utf8Error),

    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("Message status: {0}")]
    InvalidMessage(#[from] MessageStatusError),

    #[error("Binary conversion error: {0}")]
    BinaryConversionError(#[from] postcard::Error),
}

type Result<T> = std::result::Result<T, ConsumerError>;

pub trait Consumer<T: FromStr>: Sync + Send + Sized + 'static {
    fn run(self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(true)?;
        let nb_workers = 4;

        let self_arc = Arc::new(self);

        let mut senders = Vec::with_capacity(nb_workers);
        // let mut validators = (0..nb_workers).map(|_| {
        //     let consumer_clone = self_arc.clone();
        //     consumer_clone.validator()
        // }).collect();

        let mut handles = Vec::with_capacity(nb_workers * 2);

        let load_counters: Vec<Arc<AtomicUsize>> = (0..nb_workers)
            .map(|_| Arc::new(AtomicUsize::new(0)))
            .collect();

        let running = Arc::new(AtomicBool::new(true));

        for (_sender_id, load_counter) in load_counters.iter().enumerate().take(nb_workers) {
            let consumer_clone = self_arc.clone();
            let (tx, handle) = consumer_clone.worker(load_counter.clone());
            senders.push(tx);
            handles.push(handle);
        }

        println!("Listening on 127.0.0.1:{}", port);

        let running_clone = running.clone();

        let join_handle = thread::spawn(move || {
            loop {
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        let worker_idx = load_counters
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, counter)| counter.load(Ordering::Relaxed))
                            .map(|(idx, _)| idx)
                            .unwrap_or(0);

                        if senders[worker_idx].send(stream).is_err() {
                            eprintln!("Failed to send to worker {}", worker_idx);
                            break;
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::WouldBlock => {}
                        _ => eprintln!("Error accepting connection: {}", e),
                    },
                }
            }
        });

        let mut signals = Signals::new([SIGINT])?;

        if signals.forever().next().is_some() {
            running.swap(false, Ordering::Relaxed);
        }

        signals.handle().close();
        let _ = join_handle.join();
        for handle in handles {
            let _ = handle.join();
        }

        Ok(())
    }

    fn worker(
        self: Arc<Self>,
        load_counter: Arc<AtomicUsize>,
    ) -> (Sender<TcpStream>, JoinHandle<()>) {
        let (tx, rx) = channel::<TcpStream>();

        let handle = thread::spawn(move || {
            while let Ok(mut stream) = rx.recv() {
                load_counter.fetch_add(1, Ordering::Relaxed);
                println!("before worker");
                let response = match self.accept(&stream) {
                    Ok(_) => Some(MessageStatus::Ack),
                    Err(err) => {
                        match err {
                            ConsumerError::InvalidMessage(message_status_error) => Some(message_status_error.into()),
                            _ => {
                                eprintln!("{}", err);
                                None
                            },
                        }
                    }
                };

                println!("icic worker");

                if let Some(status) = response {
                    let _ = self.write_response(&mut stream, status).inspect_err(|err| eprintln!("{}", err));
                }

                load_counter.fetch_sub(1, Ordering::Relaxed);
            }
        });
        (tx, handle)
    }

    fn validator(self: Arc<Self>) -> (Sender<(SocketAddr, MessageStatus)>, JoinHandle<()>) {
        let (tx, rx) = channel::<(SocketAddr, MessageStatus)>();

        let handle = thread::spawn(move || {
            while let Ok((socket_addr, _status)) = rx.recv() {
                let _stream = TcpStream::connect(socket_addr);
            }
        });

        (tx, handle)
    }

    fn accept(&self, stream: &TcpStream) -> Result<()> {
        let mut buf_reader = std::io::BufReader::new(stream);
        let mut buf = Vec::new();
        println!("before read accept");
        buf_reader.read_to_end(&mut buf)?;


        if buf.len() < 2 {
            bail!(ConsumerError::InvalidMessageFormat(
                "Buffer too small: expected at least 2 bytes for topic length".to_string(),
            ));
        }

        let topic_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;

        if buf.len() < 2 + topic_len {
            bail!(ConsumerError::InvalidMessageFormat(format!(
                "Buffer too small: expected {} bytes for topic, got {}",
                2 + topic_len,
                buf.len()
            )));
        }

        let topic = std::str::from_utf8(&buf[2..2 + topic_len])?;

        let payload_start = 2 + topic_len;

        if buf.len() < payload_start + 4 {
            bail!(ConsumerError::InvalidMessageFormat(format!(
                "Buffer too small: expected {} bytes for payload length",
                payload_start + 4
            )));
        }

        let payload_len = u32::from_be_bytes([
            buf[payload_start],
            buf[payload_start + 1],
            buf[payload_start + 2],
            buf[payload_start + 3],
        ]) as usize;

        if buf.len() < payload_start + 4 + payload_len {
            bail!(ConsumerError::InvalidMessageFormat(format!(
                "Buffer too small: expected {} bytes for payload, got {}",
                payload_start + 4 + payload_len,
                buf.len()
            )));
        }

        let topic_variant =
            T::from_str(topic).map_err(|_| MessageStatusError::UnknownTopic(topic.to_string()))?;

        println!("ici after variant");

        let payload_bytes = &buf[payload_start + 4..payload_start + 4 + payload_len];
        self.dispatch(topic_variant, payload_bytes)
    }

    fn write_response(&self, stream: &mut self::TcpStream, status: MessageStatus) -> Result<()> {
        println!("write {:?}", status);
        postcard::to_io(&status, stream)?;
        Ok(())
    }

    fn dispatch(&self, topic: T, payload: &[u8]) -> Result<()>;
}
