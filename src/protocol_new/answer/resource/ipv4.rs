use crate::protocol_new::answer::resource::Resource;
use crate::protocol_new::answer::resource::cname::CnameResource;
use crate::cursor::Cursor;
use std::net::Ipv4Addr;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Ipv4Resource {
    pub name: String,
    pub ttl: u32,
    pub data: Ipv4Addr,
}

impl Resource<Ipv4Addr> for Ipv4Resource {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_ttl(&self) -> u32 {
        self.ttl
    }

    fn get_data(&self) -> &Ipv4Addr {
        &self.data
    }
}

impl Ipv4Resource {
    pub fn from(name: String, ttl: u32, cursor: &mut Cursor<u8>) -> Self {
        let _data_len = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let data = Ipv4Addr::new(cursor.take(), cursor.take(), cursor.take(), cursor.take());
        Ipv4Resource {
            name,
            ttl,
            data,
        }
    }
}