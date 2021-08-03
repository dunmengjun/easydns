// use std::process;
// use std::sync::{Mutex, Arc};
//
// pub const SIGINT: i32 = 2;
// pub const SIGTERM: i32 = 15;
//
// struct AbortActions<F> {
//     inner: Arc<Mutex<AbortActionInner<F>>>,
// }
//
// struct AbortActionInner<F> {
//     actions: Vec<F>,
//     is_come: bool,
// }
//
// unsafe impl<F: Fn() + Sync + Send + 'static> Sync for AbortActions<F> {}
//
// impl<F: Fn() + Sync + Send + 'static> AbortActions<F> {
//     fn new() -> Self <F> {
//         AbortActions {
//             inner: Arc::new(Mutex::new(AbortActionInner { actions: vec![], is_come: false })),
//         }
//     }
//
//     fn add_action(&self, f: F) {
//         self.actions.lock().unwrap().push(f);
//     }
//
//     fn register(&self) {
//         let arc = self.inner.clone();
//         let handle_action = move || {
//             arc.l
//         };
//         unsafe {
//             signal_hook_registry::register(SIGINT, handle_action);
//             signal_hook_registry::register(SIGTERM, handle_action);
//         };
//     }
// }
//
// pub fn register<F, const N: usize>(signals: [i32; N], action: F)
//     where F: Fn() + Sync + Send + 'static, {
//     unsafe {
//         for signal in signals {
//             signal_hook_registry::register(signal, move || {})
//         }
//     };
// }