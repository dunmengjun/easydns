use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = core::result::Result<T, Box<dyn Error>>;

pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
}