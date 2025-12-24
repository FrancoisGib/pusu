#[macro_export]
macro_rules! bail {
    ($self:expr) => {
        return Err($self)
    };
}
