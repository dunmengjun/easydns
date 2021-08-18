use crate::system::default_value;
use std::cell::RefCell;

mod array_buf;

pub type ArrayBuf<R> = Box<dyn Array<R>>;

pub struct Cursor<R> {
    array: ArrayBuf<R>,
    current: RefCell<usize>,
}

pub trait Array<T>: Send + Sync {
    fn get(&self, index: usize) -> T;
    fn get_slice(&self, start: usize, end: usize) -> &[T];
}

impl<R> Cursor<R> {
    pub fn form(array: ArrayBuf<R>) -> Self {
        Cursor {
            array,
            current: RefCell::new(0),
        }
    }

    pub fn at(&self, index: usize) {
        *self.current.borrow_mut() = index;
    }

    #[inline]
    pub fn tmp_at<F: FnMut(&Self)>(&self, index: usize, mut func: F) {
        let current_index_saved = self.get_current_index();
        self.at(index);
        func(self);
        self.at(current_index_saved);
    }

    pub fn take(&self) -> R {
        let current = *self.current.borrow();
        let result = self.array.get(current);
        *self.current.borrow_mut() = current + 1;
        result
    }

    pub fn move_to(&self, step: usize) {
        let current = *self.current.borrow();
        *self.current.borrow_mut() = current + step;
    }

    pub fn peek(&self) -> R {
        self.array.get(*self.current.borrow())
    }

    pub fn get_current_index(&self) -> usize {
        *self.current.borrow()
    }

    pub fn take_slice(&self, len: usize) -> &[R] {
        let current = *self.current.borrow();
        let result = self.array.get_slice(current, current + len);
        *self.current.borrow_mut() = current + len;
        result
    }

    pub fn take_bytes<const N: usize>(&self) -> [R; N] where R: Default + Copy {
        let mut k = default_value();
        (0..N).into_iter().for_each(|index| {
            k[index] = self.take();
        });
        k
    }
}