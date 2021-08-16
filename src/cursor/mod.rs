mod array_buf;

pub type ArrayBuf<R> = Box<dyn Array<R>>;

pub struct Cursor<R> {
    array: ArrayBuf<R>,
    current: usize,
}

pub trait Array<T>: Send + Sync {
    fn get(&self, index: usize) -> T;
    fn get_slice(&self, start: usize, end: usize) -> &[T];
}

impl<R> Cursor<R> {
    pub fn form(array: ArrayBuf<R>) -> Self {
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

    pub fn take_bytes<const N: usize>(&mut self) -> [R; N] where R: Default + Copy {
        let mut k = [R::default(); N];
        (0..N).into_iter().for_each(|index| {
            k[index] = self.take();
        });
        k
    }
}