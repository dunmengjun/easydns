use std::thread;
use crossbeam_channel::{Sender, Receiver, bounded};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub trait Task {
    fn run(&self);
}

pub struct TaskScheduler<T> {
    t_num: usize,
    sender: Sender<T>,
    receiver: Receiver<T>,
    c_num: Arc<AtomicUsize>,
}

impl<T: 'static + Task + Send> TaskScheduler<T> {
    pub fn from(t_num: usize) -> Self {
        let (sender, receiver) = bounded::<T>(1000);
        let scheduler = TaskScheduler {
            t_num,
            sender,
            receiver,
            c_num: Arc::new(AtomicUsize::new(0)),
        };
        scheduler.start();
        scheduler
    }

    fn is_thread_full(&self) -> bool {
        self.is_started() && self.c_num.load(Ordering::SeqCst) >= self.t_num
    }

    fn is_need_increase_thread(&self) -> bool {
        (!self.is_thread_full()) && self.sender.len() > 50
    }

    pub fn publish(&mut self, task: T) {
        self.sender.send(task).unwrap();
        if self.is_need_increase_thread() {
            self.create_helper_thread();
        }
    }

    fn create_main_thread(&self) {
        let receiver = self.receiver.clone();
        let c_num = self.c_num.clone();
        thread::spawn(move || {
            c_num.fetch_add(1, Ordering::SeqCst);
            loop {
                match receiver.recv() {
                    Ok(task) => task.run(),
                    Err(_e) => break
                }
            }
            c_num.fetch_sub(1, Ordering::SeqCst);
        });
    }

    fn create_helper_thread(&self) {
        let receiver = self.receiver.clone();
        let c_num = self.c_num.clone();
        thread::spawn(move || {
            c_num.fetch_add(1, Ordering::SeqCst);
            loop {
                match receiver.recv_timeout(Duration::from_secs(60)) {
                    Ok(task) => task.run(),
                    Err(_e) => break
                }
            }
            c_num.fetch_sub(1, Ordering::SeqCst);
        });
    }

    fn is_started(&self) -> bool {
        self.c_num.load(Ordering::SeqCst) > 0
    }

    fn start(&self) {
        if self.is_started() {
            return;
        }
        self.create_main_thread();
    }
}