use crate::protocol_new::answer::resource::Resource;
use crate::cursor::Cursor;
use crate::protocol_new::unzip_domain;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SoaResource {
    name: String,
    pub ttl: u32,
    data: Soa,
}

impl Resource<Soa> for SoaResource {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_ttl(&self) -> u32 {
        self.ttl
    }

    fn get_data(&self) -> &Soa {
        &self.data
    }
}

impl SoaResource {
    pub fn from(name: String, ttl: u32, cursor: &mut Cursor<u8>) -> Self {
        let _data_len = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let data = Soa::from(cursor);
        SoaResource {
            name,
            ttl,
            data,
        }
    }

    pub fn new_wit_default_soa(name: String, ttl: u32) -> Self {
        SoaResource {
            name,
            ttl,
            data: Soa::default(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Soa {
    name_server: String,
    mailbox: String,
    serial_number: u32,
    interval_refresh: u32,
    interval_retry: u32,
    expire_limit: u32,
    minimum_ttl: u32,
}

impl Soa {
    fn from(cursor: &mut Cursor<u8>) -> Self {
        let name_server = unzip_domain(cursor);
        let mailbox = unzip_domain(cursor);
        let serial_number = u32::from_be_bytes(cursor.take_bytes());
        let interval_refresh = u32::from_be_bytes(cursor.take_bytes());
        let interval_retry = u32::from_be_bytes(cursor.take_bytes());
        let expire_limit = u32::from_be_bytes(cursor.take_bytes());
        let minimum_ttl = u32::from_be_bytes(cursor.take_bytes());
        Soa {
            name_server,
            mailbox,
            serial_number,
            interval_refresh,
            interval_retry,
            expire_limit,
            minimum_ttl,
        }
    }

    fn default() -> Self {
        Soa {
            name_server: "dns17.hichina.com".to_string(),
            mailbox: "hostmaster.hichina.com".to_string(),
            serial_number: 1,
            interval_refresh: 3600,
            interval_retry: 1200,
            expire_limit: 3600,
            minimum_ttl: 600,
        }
    }
}
