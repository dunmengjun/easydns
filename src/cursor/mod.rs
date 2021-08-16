pub struct Cursor<R> {
    array: Box<dyn Array<R>>,
    current: usize,
}

pub trait Array<T> {
    fn get(&self, index: usize) -> T;
    fn get_slice(&self, start: usize, end: usize) -> &[T];
}

impl<T> Array<T> for Vec<T> where T: Clone + Copy {
    fn get(&self, index: usize) -> T {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[T] {
        &self[start..end]
    }
}

impl<R> Cursor<R> {
    pub fn form(array: Box<dyn Array<R>>) -> Self {
        Cursor {
            array,
            current: 0,
        }
    }

    pub fn at(&mut self, index: usize) {
        self.current = index;
    }

    #[inline]
    pub fn tmp_at<F: FnMut(&mut Self)>(&mut self, index: usize, mut func: F) {
        let current_index_saved = self.get_current_index();
        self.at(index);
        func(self);
        self.at(current_index_saved);
    }

    pub fn take(&mut self) -> R {
        let result = self.array.get(self.current);
        self.current += 1;
        result
    }

    pub fn move_to(&mut self, step: usize) {
        self.current += step;
    }

    pub fn peek(&mut self) -> R {
        self.array.get(self.current)
    }

    pub fn get_current_index(&self) -> usize {
        self.current
    }

    pub fn take_slice(&mut self, len: usize) -> &[R] {
        let result = self.array.get_slice(self.current, self.current + len);
        self.current += len;
        result
    }
}