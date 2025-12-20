use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use anyhow::Result;
pub use pusu_consumer_macro::consumer;
use signal_hook::{consts::SIGINT, iterator::Signals};

pub trait Consumer: Sync + Send + Sized + 'static {
    fn run(self, port: u16) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(true)?;
        let nb_workers = 4;

        let mut senders = Vec::with_capacity(nb_workers);
        let mut handles = Vec::with_capacity(nb_workers);

        let load_counters: Vec<Arc<AtomicUsize>> = (0..nb_workers)
            .map(|_| Arc::new(AtomicUsize::new(0)))
            .collect();

        let running = Arc::new(AtomicBool::new(true));

        let self_arc = Arc::new(self);

        for worker_id in 0..nb_workers {
            let consumer_clone = self_arc.clone();
            let (tx, handle) = consumer_clone.worker(worker_id, load_counters[worker_id].clone());
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
        for _ in signals.forever() {
            running.swap(false, Ordering::Relaxed);
            break;
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
        id: usize,
        load_counter: Arc<AtomicUsize>,
    ) -> (Sender<TcpStream>, JoinHandle<()>) {
        let (tx, rx) = channel::<TcpStream>();

        let handle = thread::spawn(move || {
            while let Ok(stream) = rx.recv() {
                load_counter.fetch_add(1, Ordering::Relaxed);

                if let Err(err) = self.accept(stream) {
                    eprintln!("Error on worker {}: {}", id, err);
                }

                load_counter.fetch_sub(1, Ordering::Relaxed);
            }
        });
        (tx, handle)
    }

    fn accept(&self, stream: TcpStream) -> Result<()> {
        let mut buf_reader = std::io::BufReader::new(stream);
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;

        if buf.len() < 2 {
            anyhow::bail!("Buffer too small: expected at least 2 bytes for topic length");
        }

        let topic_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;

        if buf.len() < 2 + topic_len {
            anyhow::bail!(
                "Buffer too small: expected {} bytes for topic, got {}",
                2 + topic_len,
                buf.len()
            );
        }

        let topic = std::str::from_utf8(&buf[2..2 + topic_len])?;

        let payload_start = 2 + topic_len;

        if buf.len() < payload_start + 4 {
            anyhow::bail!(
                "Buffer too small: expected {} bytes for payload length",
                payload_start + 4
            );
        }

        let payload_len = u32::from_be_bytes([
            buf[payload_start],
            buf[payload_start + 1],
            buf[payload_start + 2],
            buf[payload_start + 3],
        ]) as usize;

        if buf.len() < payload_start + 4 + payload_len {
            anyhow::bail!(
                "Buffer too small: expected {} bytes for payload, got {}",
                payload_start + 4 + payload_len,
                buf.len()
            );
        }

        let payload_bytes = &buf[payload_start + 4..payload_start + 4 + payload_len];

        self.dispatch(topic, payload_bytes)
    }

    fn dispatch(&self, topic: &str, payload: &[u8]) -> Result<()>;
}
