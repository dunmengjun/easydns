use crate::protocol::answer::resource::{Resource, BasicData};
use crate::cursor::Cursor;
use crate::protocol::{unzip_domain, wrap_name};
use crate::protocol::answer::resource::basic;

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
    pub fn create(mut basic: BasicData, cursor: &Cursor<u8>) -> Self {
        let data = Soa::from(cursor);
        basic.set_data_len(data.len as u16);
        SoaResource {
            basic,
            data,
        }
    }

    pub fn new_with_default_soa(name: String, ttl: u32) -> Self {
        let soa = Soa::default();
        let basic = basic::Builder::new()
            ._type(6)
            .ttl(ttl)
            .name(name)
            .data_len(soa.len as u16)
            .build();
        SoaResource {
            basic,
            data: soa,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.basic.set_name(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NameServer {
    domain: String,
    len: usize,
}

impl From<&Cursor<u8>> for NameServer {
    fn from(cursor: &Cursor<u8>) -> Self {
        if cursor.peek() == 0 {
            cursor.take();
            NameServer {
                domain: ".".to_string(),
                len: 1,
            }
        } else {
            let domain = unzip_domain(cursor);
            let len = domain.len() + 2;
            NameServer {
                domain,
                len,
            }
        }
    }
}

impl From<&str> for NameServer {
    fn from(str: &str) -> Self {
        NameServer {
            domain: str.to_string(),
            len: 2 + str.len(),
        }
    }
}

impl From<&NameServer> for Vec<u8> {
    fn from(name_server: &NameServer) -> Self {
        let mut vec = Vec::new();
        if name_server.domain.eq(".") {
            vec.push(0u8);
        } else {
            vec.extend(wrap_name(&name_server.domain));
        }
        vec
    }
}

impl NameServer {
    fn len(&self) -> usize {
        self.len
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Soa {
    name_server: NameServer,
    mailbox: String,
    serial_number: u32,
    interval_refresh: u32,
    interval_retry: u32,
    expire_limit: u32,
    minimum_ttl: u32,
    len: usize,
}

impl From<&Soa> for Vec<u8> {
    fn from(s: &Soa) -> Self {
        let server = &s.name_server;
        let mut vec: Vec<u8> = server.into();
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
        let name_server = NameServer::from(cursor);
        let mailbox = unzip_domain(cursor);
        let serial_number = u32::from_be_bytes(cursor.take_bytes());
        let interval_refresh = u32::from_be_bytes(cursor.take_bytes());
        let interval_retry = u32::from_be_bytes(cursor.take_bytes());
        let expire_limit = u32::from_be_bytes(cursor.take_bytes());
        let minimum_ttl = u32::from_be_bytes(cursor.take_bytes());
        let len = name_server.len() + mailbox.len() + 2 + 20;
        Soa {
            name_server,
            mailbox,
            serial_number,
            interval_refresh,
            interval_retry,
            expire_limit,
            minimum_ttl,
            len,
        }
    }

    fn default() -> Self {
        let name_server = NameServer::from("dns17.hichina.com");
        let len = name_server.len();
        Soa {
            name_server,
            mailbox: "hostmaster.hichina.com".to_string(),
            serial_number: 1,
            interval_refresh: 3600,
            interval_retry: 1200,
            expire_limit: 3600,
            minimum_ttl: 600,
            len: len + 24 + 20,
        }
    }
}
