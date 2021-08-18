use crate::protocol_new::answer::resource::{Resource, BasicData};
use crate::protocol_new::answer::resource::cname::CnameResource;
use crate::cursor::Cursor;
use std::net::Ipv4Addr;

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
    fn from(mut r: &Ipv4Resource) -> Self {
        let data = &r.basic;
        let mut vec: Vec<u8> = data.into();
        vec.extend(&r.data.octets());
        vec
    }
}

impl Ipv4Resource {
    pub fn from(basic: BasicData, cursor: &Cursor<u8>) -> Self {
        let data = Ipv4Addr::from(cursor.take_bytes());
        Ipv4Resource {
            basic,
            data,
        }
    }
}