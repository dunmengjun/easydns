use std::process;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::error::Error;

pub type Result<T> = core::result::Result<T, Box<dyn Error>>;
pub type AbortFunc = Box<dyn Fn() + Sync + Send + 'static>;

pub const SIGINT: i32 = 2;
pub const SIGTERM: i32 = 15;

static ABORT_COME: AtomicBool = AtomicBool::new(false);

pub fn register_abort_action<const N: usize>(actions: [AbortFunc; N]) {
    let actions1 = Arc::new(actions);
    let actions2 = actions1.clone();
    unsafe {
        signal_hook_registry::register(SIGINT, move || {
            run_actions(actions1.clone());
            process::exit(0);
        }).unwrap();
        signal_hook_registry::register(SIGTERM, move || {
            run_actions(actions2.clone());
            process::exit(0);
        }).unwrap();
    };
}

fn run_actions<const N: usize>(actions: Arc<[AbortFunc; N]>) {
    if ABORT_COME.load(Ordering::Relaxed) {
        return;
    }
    ABORT_COME.fetch_or(true, Ordering::SeqCst);
    for action in actions.iter() {
        action();
    }
}