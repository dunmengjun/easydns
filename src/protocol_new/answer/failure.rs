use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::header::Header;
use std::fmt::{Display, Formatter};
use std::any::Any;

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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl FailureAnswer {
    pub fn from(header: Header, question: Question) -> Self {
        FailureAnswer {
            header,
            question,
        }
    }
}