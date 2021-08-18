use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;

pub struct NoSuchNameAnswer {
    header: Header,
    question: Question,
}

impl Display for NoSuchNameAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(NO_SUCH_NAME, {})", self.question.name)
    }
}

impl Answer for NoSuchNameAnswer {
    fn to_cache(&self) -> Option<CacheRecord> {
        todo!()
    }

    fn to_bytes(&self) -> &[u8] {
        todo!()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }

    fn set_id(&mut self, id: u16) {
        self.header.id = id;
    }

    fn get_id(&self) -> &u16 {
        &self.header.id
    }
}

impl NoSuchNameAnswer {
    pub fn from(header: Header, question: Question) -> Self {
        NoSuchNameAnswer {
            header,
            question,
        }
    }
}