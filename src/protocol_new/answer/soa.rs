use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::answer::resource::{SoaResource};
use std::fmt::{Display, Formatter};
use std::any::Any;

pub struct SoaAnswer {
    header: Header,
    question: Question,
    resource: SoaResource,
}

impl Display for SoaAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(SOA, {}, {})", self.question.name, self.resource.ttl)
    }
}

impl Answer for SoaAnswer {
    fn to_cache(&self) -> Option<CacheRecord> {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SoaAnswer {
    pub fn from(header: Header, question: Question, resource: SoaResource) -> Self {
        SoaAnswer {
            header,
            question,
            resource,
        }
    }
}