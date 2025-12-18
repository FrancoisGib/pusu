use pusu::partitionable::Partitionable;

pub struct Test {
    message: String
}

fn main() -> Result<(), String> {
    let str = "salut les cocos".to_string();
    let partitions: Vec<_> = str.partition().unwrap().collect();
    println!("{:?}", partitions);
    let reconstruct  = String::reconstruct_from_vec(partitions);
    println!("{}", reconstruct.unwrap());
    Ok(())
}