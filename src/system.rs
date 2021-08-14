use std::error::Error;
use std::time::{Duration};
use std::sync::atomic::{AtomicU16, Ordering};
use crate::config::Config;
use log::LevelFilter;
use std::str::FromStr;
use std::fmt::{Debug, Formatter, Display};
use std::cell::RefCell;

pub type Result<T> = core::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct FileNotFoundError {
    pub path: String,
    pub supper: Box<dyn Error>,
}

impl Display for FileNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SuperError is here!")
    }
}

impl Error for FileNotFoundError {}

pub struct TimeNow {
    #[cfg(test)] timestamp: u128,
    add_duration: Option<Duration>,
    sub_duration: Option<Duration>,
}

impl TimeNow {
    #[cfg(test)]
    pub fn get(&mut self) -> u128 {
        let add = self.add_duration.take().map(|d| d.as_millis()).unwrap_or(0);
        let sub = self.sub_duration.take().map(|d| d.as_millis()).unwrap_or(0);
        self.timestamp + add - sub
    }
    #[cfg(test)]
    pub fn set_timestamp(&mut self, timestamp: u128) -> &mut Self {
        self.timestamp = timestamp;
        self
    }

    #[cfg(test)]
    pub fn new() -> Self {
        TimeNow {
            timestamp: 0,
            add_duration: None,
            sub_duration: None,
        }
    }

    #[cfg(not(test))]
    pub fn get(&mut self) -> u128 {
        let current_time = get_timestamp();
        let add = self.add_duration.take().map(|d| d.as_millis()).unwrap_or(0);
        let sub = self.sub_duration.take().map(|d| d.as_millis()).unwrap_or(0);
        current_time + add - sub
    }

    #[cfg(not(test))]
    pub fn new() -> Self {
        TimeNow {
            add_duration: None,
            sub_duration: None,
        }
    }

    pub fn sub(&mut self, d: Duration) -> &mut Self {
        self.sub_duration = Some(d);
        self
    }
}

thread_local! {
    pub static TIME: RefCell<TimeNow> = RefCell::new(TimeNow::new());
}

pub fn get_now() -> u128 {
    TIME.with(|r| {
        r.borrow_mut().get()
    })
}

pub fn get_sub_now(d: Duration) -> u128 {
    TIME.with(|r| {
        r.borrow_mut().sub(d).get()
    })
}

#[cfg(not(test))]
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use std::future::Future;

#[cfg(not(test))]
fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
}

#[cfg(test)]
pub fn set_time_base(base: u128) {
    TIME.with(|r| {
        r.borrow_mut().set_timestamp(base);
    })
}

static ID: AtomicU16 = AtomicU16::new(0);

pub fn next_id() -> u16 {
    match ID.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |x| {
        if x > u16::MAX - 10000 {
            Some(0);
        }
        Some(x + 1)
    }) {
        Ok(id) => id,
        Err(e) => e
    }
}

pub fn setup_panic_hook() {
    //设置panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        error!("panic message: {:?}, location in {:?}", panic_info.message(), panic_info.location());
    }));
}

pub fn setup_log_level(config: &Config) -> Result<()> {
    let level = LevelFilter::from_str(&config.log_level)?;
    log::set_max_level(level);
    Ok(())
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(move || {
        Handle::current().block_on(async {
            future.await
        })
    })
}