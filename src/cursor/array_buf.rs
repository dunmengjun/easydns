use crate::cursor::{Array, ArrayBuf};

impl Array<u8> for Vec<u8> {
    fn get(&self, index: usize) -> u8 {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self[start..end]
    }
}

impl From<Vec<u8>> for ArrayBuf<u8> {
    fn from(buf: Vec<u8>) -> Self {
        Box::new(buf)
    }
}

impl Array<u8> for [u8; 512] {
    fn get(&self, index: usize) -> u8 {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self[start..end]
    }
}

impl From<[u8; 512]> for ArrayBuf<u8> {
    fn from(buf: [u8; 512]) -> Self {
        Box::new(buf)
    }
}

impl Array<u8> for [u8; 256] {
    fn get(&self, index: usize) -> u8 {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self[start..end]
    }
}

impl From<[u8; 256]> for ArrayBuf<u8> {
    fn from(buf: [u8; 256]) -> Self {
        Box::new(buf)
    }
}