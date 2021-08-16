use crate::cursor::{Array, ArrayBuf};
use crate::system::{QueryBuf, AnswerBuf};

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

impl Array<u8> for AnswerBuf {
    fn get(&self, index: usize) -> u8 {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self[start..end]
    }
}

impl From<AnswerBuf> for ArrayBuf<u8> {
    fn from(buf: AnswerBuf) -> Self {
        Box::new(buf)
    }
}

impl Array<u8> for QueryBuf {
    fn get(&self, index: usize) -> u8 {
        self[index]
    }

    fn get_slice(&self, start: usize, end: usize) -> &[u8] {
        &self[start..end]
    }
}

impl From<QueryBuf> for ArrayBuf<u8> {
    fn from(buf: QueryBuf) -> Self {
        Box::new(buf)
    }
}