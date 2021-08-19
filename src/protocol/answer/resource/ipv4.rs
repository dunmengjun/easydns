use crate::protocol::answer::resource::{Resource, BasicData};
use crate::cursor::Cursor;
use std::net::Ipv4Addr;
use crate::cache::{IpCacheRecord, CacheItem};
use crate::protocol::answer::resource::basic::Builder;
use crate::system::get_now;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Ipv4Resource {
    basic: BasicData,
    pub data: Ipv4Addr,
}

impl Resource<Ipv4Addr> for Ipv4Resource {
    fn get_name(&self) -> &String {
        self.basic.get_name()
    }

    fn get_ttl(&self) -> u32 {
        self.basic.get_ttl()
    }

    fn get_data(&self) -> &Ipv4Addr {
        &self.data
    }
}

impl From<&Ipv4Resource> for Vec<u8> {
    fn from(r: &Ipv4Resource) -> Self {
        let data = &r.basic;
        let mut vec: Vec<u8> = data.into();
        vec.extend(&r.data.octets());
        vec
    }
}

impl From<&IpCacheRecord> for Ipv4Resource {
    fn from(record: &IpCacheRecord) -> Self {
        let basic = Builder::new()
            .name(record.get_key().clone())
            .ttl((record.get_remain_time(get_now()) / 1000) as u32)
            ._type(1)
            .data_len(4)
            .build();
        Ipv4Resource {
            basic,
            data: record.get_address().clone(),
        }
    }
}

impl Ipv4Resource {
    pub fn create(basic: BasicData, cursor: &Cursor<u8>) -> Self {
        let data = Ipv4Addr::from(cursor.take_bytes());
        Ipv4Resource {
            basic,
            data,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.basic.set_name(name);
    }
}