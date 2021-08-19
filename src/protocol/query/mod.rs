use crate::protocol::basic::BasicData;
use crate::protocol::{basic};
use crate::system::{QueryBuf, next_id};
use crate::cursor::Cursor;

const QUERY_ONLY_RECURSIVELY: u16 = 0x0100;
const QUERY_RECURSIVELY_AD: u16 = 0x0120;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DnsQuery {
    basic: BasicData,
}

impl DnsQuery {
    pub fn get_id(&self) -> u16 {
        self.basic.get_id()
    }
    pub fn set_id(&mut self, id: u16) {
        self.basic.set_id(id)
    }
    pub fn get_name(&self) -> &String {
        self.basic.get_name()
    }

    pub fn is_supported(&self) -> bool {
        let flags = self.basic.get_flags();
        flags == QUERY_ONLY_RECURSIVELY || flags == QUERY_RECURSIVELY_AD
    }
}

impl From<QueryBuf> for DnsQuery {
    fn from(buf: QueryBuf) -> Self {
        let cursor = Cursor::form(buf.into());
        DnsQuery {
            basic: BasicData::from(&cursor)
        }
    }
}

impl From<&str> for DnsQuery {
    fn from(domain: &str) -> Self {
        let basic = basic::Builder::new()
            .id(next_id())
            .name(domain.to_string())
            .flags(QUERY_ONLY_RECURSIVELY)
            .build();
        DnsQuery {
            basic
        }
    }
}

impl From<&DnsQuery> for Vec<u8> {
    fn from(query: &DnsQuery) -> Self {
        let data = &query.basic;
        data.into()
    }
}