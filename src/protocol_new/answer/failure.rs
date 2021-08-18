use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::header::Header;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;

pub struct FailureAnswer {
    header: Header,
    question: Question,
}

impl Display for FailureAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FAILURE, {})", self.question.name)
    }
}

impl Answer for FailureAnswer {
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

impl FailureAnswer {
    pub fn from(header: Header, question: Question) -> Self {
        FailureAnswer {
            header,
            question,
        }
    }

    pub fn new(id: u16, name: String) -> Self {
        FailureAnswer {
            header: Header {
                id,
                flags: 0x8182,
                question_count: 1,
                answer_count: 0,
                authority_count: 0,
                additional_count: 0,
            },
            question: Question {
                name,
                _type: 1,
                class: 1,
            },
        }
    }
}