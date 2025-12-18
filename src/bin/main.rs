use pusu::{
    broker::broker,
    consumer::{Consumer, consumer},
};
use serde::Deserialize;

#[broker]
struct MyBroker {
    user: User,
}

#[consumer]
struct MyConsumer {
    #[topic("user_handler")]
    user: User,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct User {
    username: String,
    age: u8,
}

fn user_handler(v: User) {
    println!(" {:?}", v);
}

fn main() -> Result<(), String> {
    let c = MyConsumer {};
    c.start(8080)?;
    Ok(())
}
