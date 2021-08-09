use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU16, Ordering};
use crate::cache;
use crate::config::Config;
use log::LevelFilter;
use std::str::FromStr;

pub type Result<T> = core::result::Result<T, Box<dyn Error>>;

pub fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
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

pub fn setup_exit_process_task() {
    //创建任务去监听ctrl_c event
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl_c event");
        cache::run_abort_action().await.unwrap();
        std::process::exit(0);
    });
}

pub fn setup_panic_hook() {
    //设置panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        error!("panic message: {:?}, location in {:?}", panic_info.message(), panic_info.location());
    }));
}

pub fn setup_log_level(config: &Config) -> Result<()> {
    let level = LevelFilter::from_str(config.log_level)?;
    log::set_max_level(level);
    Ok(())
}