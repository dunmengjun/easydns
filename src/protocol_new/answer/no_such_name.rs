use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use std::fmt::{Display, Formatter};
use std::any::Any;

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

    fn as_any(&self) -> &dyn Any {
        self
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