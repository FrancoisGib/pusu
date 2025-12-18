pub use consumer_macro::consumer;

pub trait Consumer {
    fn start(&self, port: u16) -> Result<(), String> {
        const TOPIC_BUFFER_CAPACITY: usize = 32;

        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .map_err(|err| err.to_string())?;

        loop {
            if let Ok((stream, _addr)) = listener.accept() {
                let mut buf_reader = std::io::BufReader::new(stream);
                let mut buf = String::with_capacity(TOPIC_BUFFER_CAPACITY);

                if std::io::BufRead::read_line(&mut buf_reader, &mut buf).is_err() {
                    continue;
                }

                if !buf.starts_with("topic: ") {
                    continue;
                }

                let topic = buf[7..].trim_end();

                let _ = self
                    .dispatch(topic, &mut buf_reader)
                    .inspect_err(|err| println!("{}", err));
            }
        }
    }

    fn dispatch(
        &self,
        topic: &str,
        stream: &mut std::io::BufReader<std::net::TcpStream>,
    ) -> Result<(), String>;
}
