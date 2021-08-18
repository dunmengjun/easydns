use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::answer::resource::{SoaResource};
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;

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

impl SoaAnswer {
    pub fn from(header: Header, question: Question, resource: SoaResource) -> Self {
        SoaAnswer {
            header,
            question,
            resource,
        }
    }

    pub fn default_soa(id: u16, name: String) -> Self {
        let header = Header {
            id,
            flags: 0x8180,
            question_count: 1,
            answer_count: 0,
            authority_count: 1,
            additional_count: 0,
        };
        let question = Question {
            name: name.clone(),
            _type: 1,
            class: 1,
        };
        SoaAnswer {
            header,
            question,
            resource: SoaResource::new_wit_default_soa(name, 600),
        }
    }
}