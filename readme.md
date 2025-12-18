# pusu

pusu is a publisher - subscriber event-driven rust crate to send different messages across topics.

It is heavily inspired by kafka but will be much lighter.
The goal of the project is to have a convenient way to configure a cluster of brokers, publishers and subscribers with rust proc macros.
I wanted to generate all the "topics" in a static way to avoid dynamic traits.

I'm doing this project mostly to learn proc macros and also because I wanted to make my own event-driven architecture.

For now, the consumers can receive messages from different topics, and serde deserialize it, the error handling is bad for now but it will be improved. 

```rs
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
```

The next things that will be implemented are the brokers, which will be sending the messages from their queues to the subscribers, and after that the producers will also be implemented.
