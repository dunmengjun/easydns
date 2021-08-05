use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU16, Ordering};

pub type Result<T> = core::result::Result<T, Box<dyn Error>>;

pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
}

static ID: AtomicU16 = AtomicU16::new(0);

pub fn next_id() -> u16 {
    ID.fetch_add(1, Ordering::SeqCst)
}