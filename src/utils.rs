use rand::{Rng, rng, distr::Alphanumeric};

pub fn generate_random_str(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()

}