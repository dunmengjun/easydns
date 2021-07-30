pub struct Cursor {
    buf: Vec<u8>,
    current: usize,
}

impl Cursor {
    pub fn from(buf: Vec<u8>) -> Self {
        Cursor {
            buf,
            current: 0,
        }
    }

    pub fn at(&mut self, index: usize) {
        self.current = index;
    }

    pub fn take(&mut self) -> u8 {
        let result: u8 = self.buf[self.current];
        self.current += 1;
        result
    }

    pub fn peek(&mut self) -> u8 {
        self.buf[self.current]
    }

    pub fn get_current_index(&self) -> usize {
        self.current
    }

    pub fn take_slice(&mut self, len: usize) -> &[u8] {
        if self.current + len > self.buf.len() {
            println!("xxx");
        }
        let result = &self.buf[self.current..self.current + len];
        self.current += len;
        result
    }
}