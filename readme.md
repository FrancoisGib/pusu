# pusu

pusu is a publisher - subscriber event-driven rust crate to send different messages across topics.

It is heavily inspired by kafka but will be much lighter.
The goal of the project is to have a convenient way to configure a cluster of brokers, publishers and subscribers with rust proc macros.
I wanted to generate all the "topics" in a static way to avoid dynamic traits.

I'm doing this project mostly to learn proc macros and also because I wanted to make my own event-driven architecture.

For now, producers can send messages to registered consumers, the initialization methods, will be improved in the future.

[Example](src/bin/main.rs)
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

    producer.user.add_receiver(id, addr);
    producer.book.add_receiver(id, addr);
    producer.count.add_receiver(id, addr);

    producer.produce_user(User { username: "Username".to_string(), age: 25 })?;
    producer.produce_book(Book { name: "Dune".to_string(), author: "Frank Herbert".to_string() })?;
    producer.produce_count()?;

    Ok(())
}
```

The next things that will be implemented are the brokers, for now producers sends directly to consumers, in the future they will be able to do both, depending if you want replication and fault tolerance.

Another feature will be configuration in yaml or toml format to register automatically the instances of the receivers.