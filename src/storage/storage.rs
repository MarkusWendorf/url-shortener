#[derive(Debug)]
pub enum Error {
    DuplicateKey,
    GenericError,
}

pub trait Storage: Send {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str) -> Result<(), Error>;
    fn key_count(&self) -> u64;
}
