use std::{thread, panic};
use crossbeam_channel::{Sender, Receiver, bounded};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::panic::{RefUnwindSafe};
use crate::error::{Result, PanicError};
use std::error::Error;

pub trait Task {
    fn run(&self) -> Result<()>;
}

pub struct TaskScheduler<T> {
    t_num: usize,
    sender: Sender<T>,
    receiver: Receiver<T>,
    c_num: Arc<AtomicUsize>,
}

impl<T: 'static + Task + Send + RefUnwindSafe> TaskScheduler<T> {
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

    pub fn publish(&mut self, task: T) -> Result<()> {
        if self.is_need_increase_thread() {
            self.create_helper_thread();
        }
        Ok(self.sender.send(task)?)
    }

    fn create_main_thread(&self) {
        let receiver = self.receiver.clone();
        let c_num = self.c_num.clone();
        thread::spawn(move || {
            c_num.fetch_add(1, Ordering::SeqCst);
            loop {
                if let Err(_) = TaskScheduler::run_task(receiver.recv()) {
                    break;
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
                if let Err(_) = TaskScheduler::run_task(
                    receiver.recv_timeout(Duration::from_secs(60))) {
                    break;
                }
            }
            c_num.fetch_sub(1, Ordering::SeqCst);
        });
    }

    fn run_task<R: 'static + Error>(result: std::result::Result<T, R>) -> Result<()> {
        if let Ok(task) = result {
            //把任务里面的panic hold住，不让线程直接挂了，导致后续线程计数没法更新
            match panic::catch_unwind(|| { task.run().unwrap() }) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("thread {:?} panic by error {:?}",
                             thread::current().id(), e);
                    Err(Box::new(PanicError))
                }
            }
        } else {
            Err(Box::new(result.err().unwrap()))
        }
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