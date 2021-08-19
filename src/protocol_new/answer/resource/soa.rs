use crate::protocol_new::answer::resource::{Resource, BasicData};
use crate::cursor::Cursor;
use crate::protocol_new::{unzip_domain, wrap_name};
use crate::protocol_new::answer::resource::basic;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SoaResource {
    basic: BasicData,
    data: Soa,
}

impl Resource<Soa> for SoaResource {
    fn get_name(&self) -> &String {
        self.basic.get_name()
    }

    fn get_ttl(&self) -> u32 {
        self.basic.get_ttl()
    }

    fn get_data(&self) -> &Soa {
        &self.data
    }
}

impl From<&SoaResource> for Vec<u8> {
    fn from(r: &SoaResource) -> Self {
        let data = &r.basic;
        let mut vec: Vec<u8> = data.into();
        let soa = &r.data;
        let data_vec: Vec<u8> = soa.into();
        vec.extend(data_vec);
        vec
    }
}

impl SoaResource {
    pub fn create(basic: BasicData, cursor: &Cursor<u8>) -> Self {
        let data = Soa::from(cursor);
        SoaResource {
            basic,
            data,
        }
    }

    pub fn new_wit_default_soa(name: String, ttl: u32) -> Self {
        let basic = basic::Builder::new()
            ._type(6)
            .ttl(ttl)
            .name(name)
            .build();
        SoaResource {
            basic,
            data: Soa::default(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Soa {
    name_server: String,
    mailbox: String,
    serial_number: u32,
    interval_refresh: u32,
    interval_retry: u32,
    expire_limit: u32,
    minimum_ttl: u32,
}

impl From<&Soa> for Vec<u8> {
    fn from(s: &Soa) -> Self {
        let mut vec = Vec::new();
        vec.extend(wrap_name(&s.name_server));
        vec.extend(wrap_name(&s.mailbox));
        vec.extend(&s.serial_number.to_be_bytes());
        vec.extend(&s.interval_refresh.to_be_bytes());
        vec.extend(&s.interval_retry.to_be_bytes());
        vec.extend(&s.expire_limit.to_be_bytes());
        vec.extend(&s.minimum_ttl.to_be_bytes());
        vec
    }
}

impl Soa {
    fn from(cursor: &Cursor<u8>) -> Self {
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
