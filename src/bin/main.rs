use std::{
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Result;
use pusu::{
    consumer::{Consumer, consumer},
    producer::{ReceiverDispatch, producer},
};
use serde::{Deserialize, Serialize};

struct AppState {
    counter: u64,
}

#[consumer]
struct MyConsumer {
    #[topic("user_handler")]
    user: User,

    #[topic("book_handler")]
    book: Book,

    #[state("state")]
    #[topic("count_handler")]
    count: (),

    state: Arc<Mutex<AppState>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    username: String,
    age: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct Book {
    name: String,
    author: String,
}

fn user_handler(v: User) {
    println!("user: {}, {}", v.username, v.age);
}

fn book_handler(v: Book) {
    println!("book: {}, {}", v.name, v.author);
}

fn count_handler(state: Arc<Mutex<AppState>>) {
    let mut lock = state.lock().unwrap();
    lock.counter += 1;
    println!("count: {}", lock.counter);
}

#[producer]
struct MyProducer {
    user: User,

    book: Book,

    count: (),
}

fn main() -> Result<()> {
    let handle = thread::spawn(|| {
        let c = MyConsumer {
            state: Arc::new(Mutex::new(AppState { counter: 0 })),
        };
        c.run(8080).unwrap();
    });

    sleep(Duration::from_millis(100));

    let mut producer = MyProducer::new();
    let id = 1;
    let addr = "localhost:8080";

    producer.add_receiver("user", id, addr)?;
    producer.add_receiver("book", id, addr)?;
    producer.add_receiver("count", id, addr)?;

    producer.produce_user(User {
        username: "Username".to_string(),
        age: 25,
    })?;
    producer.produce_book(Book {
        name: "Dune".to_string(),
        author: "Frank Herbert".to_string(),
    })?;
    producer.produce_count()?;

    handle.join().unwrap();
    Ok(())
}
