mod failure;
mod resource;
mod no_such_name;
mod soa;
mod ipv4;

use crate::cache::CacheRecord;
use crate::protocol_new::question::Question;
use crate::system::AnswerBuf;
use crate::cursor::Cursor;
use crate::protocol_new::header::Header;
use crate::protocol_new::answer::resource::{CnameResource, Ipv4Resource, SoaResource};
use crate::protocol_new::answer::no_such_name::NoSuchNameAnswer;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol::DNSQuery;

pub type DnsAnswer = Box<dyn Answer>;

pub use ipv4::Ipv4Answer;
pub use failure::FailureAnswer;
pub use soa::SoaAnswer;

pub trait Answer: Display + Send + Sync {
    fn to_cache(&self) -> Option<CacheRecord>;
    fn to_bytes(&self) -> &[u8];
    fn as_any(&self) -> &(dyn Any + Send + Sync);
    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);
    fn set_id(&mut self, id: u16);
    fn get_id(&self) -> &u16;
}

impl From<AnswerBuf> for DnsAnswer {
    fn from(buf: AnswerBuf) -> Self {
        let mut cursor = Cursor::form(buf.into());
        let mut header = Header::from(&mut cursor);
        if header.answer_count == 0 && header.authority_count == 0 {
            header.flags = 0x8182;
        }
        if header.question_count > 1 {
            panic!("不支持一个请求里有多个域名查询")
        }
        let question = Question::from(&mut cursor);
        if header.flags == 0x8182 {
            return DnsAnswer::from(FailureAnswer::from(header, question));
        }
        if header.flags == 0x8183 {
            return DnsAnswer::from(NoSuchNameAnswer::from(header, question));
        }
        let mut ipv4_records = Vec::new();
        (0..header.answer_count as usize).into_iter().for_each(|_| {
            let temp = Question::from(&mut cursor);
            let ttl = u32::from_be_bytes([cursor.take(),
                cursor.take(), cursor.take(), cursor.take()]);
            if temp._type == 5 {
                // cname记录 目前的处理是移除
                let _resource = CnameResource::from(temp.name.clone(), ttl, &mut cursor);
            } else if temp._type == 1 {
                // a记录
                ipv4_records.push(Ipv4Resource::from(question.name.clone(), ttl, &mut cursor));
            } else {
                panic!("不支持的应答资源记录类型: name = {}, type = {}", temp.name, temp._type)
            };
        });
        let mut soa_records = Vec::new();
        (0..header.authority_count as usize).into_iter().for_each(|_| {
            let temp = Question::from(&mut cursor);
            let ttl = u32::from_be_bytes([cursor.take(),
                cursor.take(), cursor.take(), cursor.take()]);
            if temp._type == 6 {
                soa_records.push(SoaResource::from(question.name.clone(), ttl, &mut cursor));
            } else {
                panic!("不支持的认证资源记录类型: name = {}, type = {}", temp.name, temp._type)
            }
        });
        if !ipv4_records.is_empty() {
            return DnsAnswer::from(Ipv4Answer::from(header, question, ipv4_records));
        }
        if !soa_records.is_empty() {
            return DnsAnswer::from(SoaAnswer::from(header, question, soa_records.remove(0)));
        }
        unreachable!()
    }
}

impl From<FailureAnswer> for DnsAnswer {
    fn from(f: FailureAnswer) -> Self {
        Box::new(f)
    }
}

impl From<NoSuchNameAnswer> for DnsAnswer {
    fn from(f: NoSuchNameAnswer) -> Self {
        Box::new(f)
    }
}

impl From<SoaAnswer> for DnsAnswer {
    fn from(f: SoaAnswer) -> Self {
        Box::new(f)
    }
}

impl From<Ipv4Answer> for DnsAnswer {
    fn from(f: Ipv4Answer) -> Self {
        Box::new(f)
    }
}