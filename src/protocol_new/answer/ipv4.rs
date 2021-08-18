use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::answer::resource::Ipv4Resource;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::{DnsAnswer};
use crate::protocol::DNSQuery;
use std::net::Ipv4Addr;

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

impl Ipv4Answer {
    pub fn from(header: Header, question: Question, resources: Vec<Ipv4Resource>) -> Self {
        Ipv4Answer {
            header,
            question,
            resources,
        }
    }

    pub fn combine(&mut self, other: DnsAnswer) {
        if let Some(answer) = other.as_any().downcast_ref::<Self>() {}
        todo!()
    }
    pub fn empty_answer(id: u16, name: String) -> Self {
        Ipv4Answer {
            header: Header {
                id,
                flags: 0x8180,
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
            resources: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    pub fn get_all_ips(&self) -> Vec<&Ipv4Addr> {
        self.resources.iter().map(|r| {
            &r.data
        }).collect()
    }

    pub fn retain_ip(&mut self, ip: &Ipv4Addr) {
        self.resources.retain(|r| {
            r.data.eq(ip)
        });
        self.header.answer_count = 1;
    }
}