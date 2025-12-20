use std::collections::VecDeque;

use super::Message;

pub fn rf() {}

pub struct Topic<T> {
    pub name: String,
    pub queue: VecDeque<Message<T>>,
    pub next_id: usize,
}

impl<T> Topic<T> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            queue: VecDeque::new(),
            next_id: 0,
        }
    }

    pub fn publish(&mut self, payload: T) {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(Message { id, payload });
    }

    pub fn consume(&mut self) -> Option<Message<T>> {
        self.queue.pop_front()
    }
}
