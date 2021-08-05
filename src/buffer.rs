pub struct PacketBuffer {
    buf: [u8; 512],
    current: usize,
}

impl PacketBuffer {
    pub fn new() -> Self {
        PacketBuffer {
            buf: [0u8; 512],
            current: 0,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf
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

    pub fn take(&mut self) -> u8 {
        let result: u8 = self.buf[self.current];
        self.current += 1;
        result
    }

    pub fn move_to(&mut self, step: usize) {
        self.current += step;
    }

    pub fn peek(&mut self) -> u8 {
        self.buf[self.current]
    }

    pub fn get_current_index(&self) -> usize {
        self.current
    }

    pub fn take_slice(&mut self, len: usize) -> &[u8] {
        let result = &self.buf[self.current..self.current + len];
        self.current += len;
        result
    }
}