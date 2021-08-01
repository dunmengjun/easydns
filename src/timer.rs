use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
}