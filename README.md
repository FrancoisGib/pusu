# pusu

pusu is a publisher - subscriber event-driven rust crate to send different messages across topics.

[![Build status](https://github.com/FrancoisGib/pusu/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/FrancoisGib/pusu/actions/workflows/ci.yml)
[![Release status](https://github.com/FrancoisGib/pusu/actions/workflows/release.yml/badge.svg?branch=main)](https://github.com/FrancoisGib/pusu/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/pusu)](https://crates.io/crates/pusu)
[![Documentation](https://docs.rs/pusu/badge.svg)](https://docs.rs/pusu/latest)

It is heavily inspired by kafka but will be much lighter.
The goal of the project is to have a convenient way to configure a cluster of brokers, publishers and subscribers with rust proc macros.
I wanted to generate all the "topics" in a static way to avoid dynamic traits.

I'm doing this project mostly to learn proc macros and also because I wanted to make my own event-driven architecture.

For now, producers can send messages to registered consumers, the initialization methods, will be improved in the future.

[Example](src/bin/main.rs)
[Config example](examples/Config.yaml)
```rs
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
    let c = MyConsumer {
        state: Arc::new(Mutex::new(AppState { counter: 0 })),
    };
    c.run(8080)?;

    let mut producer = MyProducer::new();
    let id = 1;
    let addr = "localhost:8080";

    producer.add_receiver(MyProducerTopic::User, id, addr);
    producer.add_receiver(MyProducerTopic::Book, id, addr);
    producer.add_receiver(MyProducerTopic::Count, id, addr);
    // or
    let mut producer = MyProducer::from_config("examples/Config.yaml")?;
    

    producer.produce_user(User { username: "Username".to_string(), age: 25 })?;
    producer.produce_book(Book { name: "Dune".to_string(), author: "Frank Herbert".to_string() })?;
    producer.produce_count()?;

    Ok(())
}
```

## TODO

- Fault tolerance and replication on brokers (with abstraction on pub sub sides)
- Logging for debugging purpose
- Async runtime with tokio, for now it is an os threads scheduling for projects without tokio