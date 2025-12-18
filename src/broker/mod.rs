use std::collections::{HashMap, VecDeque};

pub struct Message<T: Sendable> {
    id: usize,
    message: T,
}
pub struct Broker {
    topic_queues: HashMap<String, VecDeque<Message<dyn Sendable>>>
}

impl Broker {
    pub fn add_partition<T: Sendable>(&mut self, topic: String, message: T) {
        self.topic_queues.entry(topic).or_default().push_back(Message { id: 10, message });
    }

    
}

pub trait Sendable {}