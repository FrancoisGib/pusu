use std::{io::Read, net::TcpStream};

use anyhow::Result;
pub use consumer_macro::consumer;

pub trait Consumer {
    fn run(&self, port: u16) -> Result<()> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))?;

        loop {
            if let Ok((stream, _addr)) = listener.accept() {
                let _ = self.accept(stream).inspect_err(|err| eprintln!("{}", err));
            }
        }
    }

    fn accept(&self, stream: TcpStream) -> Result<()> {
        let mut buf_reader = std::io::BufReader::new(stream);
        let mut buf = Vec::new();

        buf_reader.read_to_end(&mut buf)?;
        let topic_len = u16::from_be_bytes(buf[..2].try_into()?) as usize;
        let topic = str::from_utf8(&buf[2..2 + topic_len])?;

        let payload_start = 2 + topic_len;
        let payload_len =
            u32::from_be_bytes(buf[payload_start..payload_start + 4].try_into()?) as usize;
        let payload_bytes = &buf[payload_start + 4..payload_start + 4 + payload_len];

        self.dispatch(topic, payload_bytes)
    }

    fn dispatch(&self, topic: &str, stream: &[u8]) -> anyhow::Result<()>;
}
