use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::answer::resource::Ipv4Resource;
use std::fmt::{Display, Formatter};
use std::any::Any;

pub struct Ipv4Answer {
    header: Header,
    question: Question,
    resources: Vec<Ipv4Resource>,
}

impl Display for Ipv4Answer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(IP, {}, {}, {})", self.question.name, self.resources[0].ttl, self.resources[0].data)
    }
}

impl Answer for Ipv4Answer {
    fn to_cache(&self) -> Option<CacheRecord> {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Ipv4Answer {
    pub fn from(header: Header, question: Question, resources: Vec<Ipv4Resource>) -> Self {
        Ipv4Answer {
            header,
            question,
            resources,
        }
    }
}