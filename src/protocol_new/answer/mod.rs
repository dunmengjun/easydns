mod failure;
mod resource;
mod no_such_name;
mod soa;
mod ipv4;

use crate::cache::CacheRecord;
use crate::system::AnswerBuf;
use crate::cursor::Cursor;
use crate::protocol_new::answer::resource::{CnameResource, Ipv4Resource, SoaResource};
use crate::protocol_new::answer::no_such_name::NoSuchNameAnswer;
use std::fmt::{Display};
use std::any::Any;

pub type DnsAnswer = Box<dyn Answer>;

pub use ipv4::Ipv4Answer;
pub use failure::FailureAnswer;
pub use soa::SoaAnswer;
use crate::protocol_new::basic::BasicData;

pub trait Answer: Display + Send + Sync {
    fn to_cache(&self) -> Option<CacheRecord>;
    fn to_bytes(&self) -> Vec<u8>;
    fn as_any(&self) -> &(dyn Any + Send + Sync);
    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);
    fn set_id(&mut self, id: u16);
    fn get_id(&self) -> u16;
}

impl From<AnswerBuf> for DnsAnswer {
    fn from(buf: AnswerBuf) -> Self {
        // info!("buf: {:?}", &buf[0..buf.len()]);
        let cursor = Cursor::form(buf.into());
        let data = BasicData::from(&cursor);
        if data.get_flags() == 0x8182 {
            return FailureAnswer::from(data).into();
        }
        if data.get_answer_count() == 0 && data.get_authority_count() == 0 {
            return FailureAnswer::from(data).into();
        }
        if data.get_flags() == 0x8183 {
            return NoSuchNameAnswer::from(data).into();
        }
        let mut ipv4_records = Vec::new();
        (0..data.get_answer_count() as usize).into_iter().for_each(|_| {
            let r_data = resource::BasicData::from(&cursor);
            if r_data.get_type() == 5 {
                // cname记录 目前的处理是移除
                let _resource = CnameResource::create(r_data, &cursor);
            } else if r_data.get_type() == 1 {
                // a记录
                ipv4_records.push(Ipv4Resource::create(r_data, &cursor));
            } else {
                panic!("不支持的应答资源记录类型: name = {}, type = {}",
                       r_data.get_name(), r_data.get_type())
            };
        });
        let mut soa_records = Vec::new();
        (0..data.get_authority_count() as usize).into_iter().for_each(|_| {
            let r_data = resource::BasicData::from(&cursor);
            if r_data.get_type() == 6 {
                soa_records.push(SoaResource::create(r_data, &cursor));
            } else {
                panic!("不支持的认证资源记录类型: name = {}, type = {}",
                       r_data.get_name(), r_data.get_type())
            }
        });
        if !ipv4_records.is_empty() {
            return Ipv4Answer::create(data, ipv4_records).into();
        }
        if !soa_records.is_empty() {
            return SoaAnswer::create(data, soa_records.remove(0)).into();
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