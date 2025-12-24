use std::{thread::sleep, time::Duration};

use anyhow::Result;
use pusu::producer::producer;
use serde::{Deserialize, Serialize};

// struct AppState {
//     counter: u64,
// }

// #[consumer]
// struct MyConsumer {
//     #[topic("user_handler")]
//     user: User,

//     #[topic("book_handler")]
//     book: Book,

//     #[state("state")]
//     #[topic("count_handler")]
//     count: (),

//     state: Arc<Mutex<AppState>>,
// }

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

// fn user_handler(v: User) {
//     println!("user: {}, {}", v.username, v.age);
// }

// fn book_handler(v: Book) {
//     println!("book: {}, {}", v.name, v.author);
// }

// fn count_handler(state: Arc<Mutex<AppState>>) {
//     let mut lock = state.lock().unwrap();
//     lock.counter += 1;
//     println!("count: {}", lock.counter);
// }

#[producer]
struct MyProducer {
    user: User,

    book: Book,

    count: (),
}

fn main() -> Result<()> {
    // let handle = thread::spawn(|| {
    //     let c = MyConsumer {
    //         state: Arc::new(Mutex::new(AppState { counter: 0 })),
    //     };
    //     c.run(8080).unwrap();
    // });

    // sleep(Duration::from_millis(100));

    // let mut producer = MyProducer::from_config("examples/Config.yaml")?;

    // for _ in 0..1000 {
    //     println!("ici");
    //     producer.user(User {
    //         username: "Username".to_string(),
    //         age: 25,
    //     })?;
    //     producer.book(Book {
    //         name: "Dune".to_string(),
    //         author: "Frank Herbert".to_string(),
    //     })?;
    //     producer.count()?;
    // }

    // handle.join().unwrap();

    let mut producer = MyProducer::from_config("examples/Config.yaml").unwrap();
    let user = User { age: 25, username: "user".into() };
    let book = Book {author: "Frank Herbert".into(), name: "Dune".into()};
    producer.run()?;
    producer.user(&user)?;
    producer.book(&book)?;
    producer.count(&())?;
    sleep(Duration::from_secs(10));
    producer.close()?;
    Ok(())
}
