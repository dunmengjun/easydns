use std::{thread};
use crossbeam_channel::{Sender, Receiver, bounded};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::cell::RefCell;
use std::thread::JoinHandle;
use crate::system::{Result, AbortFunc};

#[derive(Debug)]
struct ThreadVec {
    handlers: RefCell<Vec<Arc<HandleContainer>>>,
}

unsafe impl Sync for ThreadVec {}

impl ThreadVec {
    fn new() -> Self {
        ThreadVec {
            handlers: RefCell::new(vec![])
        }
    }
    fn push(&self, thread: Arc<HandleContainer>) {
        self.handlers.borrow_mut().push(thread);
    }

    fn join_all(&self) {
        self.handlers.borrow().iter()
            .filter(|c| { !c.is_empty() })
            .map(|c| c.take_thread())
            .for_each(|option| {
                if let Some(handler) = option {
                    let id = handler.thread().id();
                    println!("has thread join {:?} started", id);
                    //如果线程中出现panic，这里就会出错
                    let _result = handler.join().map(|_| {
                        println!("has thread join {:?} ended", id);
                    }).map_err(|e| {
                        println!("join has error: {:?}", e);
                    });
                }
            });
    }
}

#[derive(Debug)]
struct HandleContainer {
    slot: RefCell<Vec<JoinHandle<()>>>,
}

unsafe impl Sync for HandleContainer {}

impl HandleContainer {
    fn new() -> Self {
        HandleContainer {
            slot: RefCell::new(Vec::with_capacity(1))
        }
    }
    fn store(&self, join: JoinHandle<()>) {
        self.slot.borrow_mut().push(join);
    }

    fn clear(&self) {
        self.slot.borrow_mut().clear();
    }

    fn is_empty(&self) -> bool {
        self.slot.borrow().is_empty()
    }

    fn take_thread(&self) -> Option<JoinHandle<()>> {
        self.slot.borrow_mut().pop()
    }
}

struct ThreadSentinel {
    container: Arc<HandleContainer>,
    c_num: Arc<AtomicUsize>,
}

impl ThreadSentinel {
    fn from(c_num: Arc<AtomicUsize>, container: Arc<HandleContainer>) -> Self {
        c_num.fetch_add(1, Ordering::SeqCst);
        ThreadSentinel {
            container,
            c_num,
        }
    }
}

impl Drop for ThreadSentinel {
    fn drop(&mut self) {
        println!("Sentinel drop started in thread: {:?}", thread::current().id());
        self.c_num.fetch_sub(1, Ordering::SeqCst);
        self.container.clear();
        println!("Sentinel dropped in thread: {:?}", thread::current().id());
    }
}

pub trait Task {
    fn run(&self) -> Result<()>;
}

pub enum TaskMsg<T> {
    Task(T),
    End,
}

pub struct TaskScheduler<T> {
    t_num: usize,
    sender: Sender<TaskMsg<T>>,
    receiver: Receiver<TaskMsg<T>>,
    c_num: Arc<AtomicUsize>,
    thread_vec: Arc<ThreadVec>,
}

impl<T: 'static + Task + Send> TaskScheduler<T> {
    pub fn from(t_num: usize) -> Self {
        let (sender, receiver) = bounded(1000);
        let scheduler = TaskScheduler {
            t_num,
            sender,
            receiver,
            c_num: Arc::new(AtomicUsize::new(0)),
            thread_vec: Arc::new(ThreadVec::new()),
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

    pub fn publish(&mut self, task: TaskMsg<T>) -> Result<()> {
        if self.is_need_increase_thread() {
            self.create_helper_thread();
        }
        Ok(self.sender.send(task)?)
    }

    fn create_main_thread(&self) {
        let receiver = self.receiver.clone();
        self.create_thread(move || {
            match receiver.recv() {
                Ok(msg) => { Ok(msg) }
                Err(e) => { Err(format!("{}", e).into()) }
            }
        });
    }

    fn create_helper_thread(&self) {
        let receiver = self.receiver.clone();
        self.create_thread(move || {
            match receiver.recv_timeout(Duration::from_secs(60)) {
                Ok(msg) => { Ok(msg) }
                Err(e) => { Err(format!("{}", e).into()) }
            }
        });
    }

    fn create_thread<F>(&self, get_msg_func: F)
        where F: Fn() -> Result<TaskMsg<T>> + Send + 'static {
        let c_num = self.c_num.clone();
        let container = Arc::new(HandleContainer::new());
        let arc_container = container.clone();
        let handler = thread::spawn(move || {
            let _sentinel = ThreadSentinel::from(c_num, arc_container);
            loop {
                match get_msg_func() {
                    Ok(task_msg) => {
                        match task_msg {
                            TaskMsg::Task(task) => { task.run().unwrap(); }
                            TaskMsg::End => break
                        }
                    }
                    Err(e) => {
                        println!("thread {:?}, recv error {}", thread::current().id(), e);
                        break;
                    }
                }
            }
        });
        container.store(handler);
        self.thread_vec.push(container);
    }

    fn is_started(&self) -> bool {
        self.c_num.load(Ordering::Relaxed) > 0
    }

    fn start(&self) {
        if self.is_started() {
            return;
        }
        self.create_main_thread();
    }

    pub fn get_abort_action(&self) -> AbortFunc {
        //设置线程池在ctrl+c信号到来时应该发送结束消息并等待所有线程完成他们的任务
        let arc_thread_vec = self.thread_vec.clone();
        let arc_sender = self.sender.clone();
        let c_num = self.c_num.clone();
        Box::new(move || {
            let num = c_num.load(Ordering::Relaxed);
            for _ in 0..num * 2 {
                arc_sender.send(TaskMsg::End).unwrap();
            }
            arc_thread_vec.join_all();
            println!("线程全部完整退出")
        })
    }
}