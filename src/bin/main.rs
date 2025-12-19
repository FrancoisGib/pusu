use std::sync::{Arc, Mutex};

use anyhow::{Result};
use pusu::{
    broker::broker,
    consumer::{Consumer, consumer},
};
use serde::Deserialize;

#[broker]
struct MyBroker {
    user: User,
}

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

#[derive(Debug, Deserialize)]
struct User {
    username: String,
    age: u8,
}

#[derive(Debug, Deserialize)]
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

fn main() -> Result<()> {
    let c = MyConsumer {
        state: Arc::new(Mutex::new(AppState { counter: 0 })),
    };
    c.start(8080)?;
    Ok(())
}
