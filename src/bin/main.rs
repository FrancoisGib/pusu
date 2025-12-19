use std::sync::{Arc, Mutex};

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
    #[state("state")]
    user: User,

    #[topic("book_handler")]
    #[state("state")]
    book: Book,

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

fn user_handler(state: Arc<Mutex<AppState>>, v: User) {
    let mut lock = state.lock().unwrap();
    println!("count: {}", lock.counter);
    lock.counter += 1;
    println!("user: {}, {}", v.username, v.age);
}

fn book_handler(state: Arc<Mutex<AppState>>, v: Book) {
    let mut lock = state.lock().unwrap();
    println!("count: {}", lock.counter);
    lock.counter += 1;
    println!("book: {}, {}", v.name, v.author);
}

fn main() -> Result<(), String> {
    let c = MyConsumer {
        state: Arc::new(Mutex::new(AppState { counter: 0 })),
    };
    c.start(8080)?;
    Ok(())
}
