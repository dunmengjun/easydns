use crate::protocol_new::question::Question;
use crate::cursor::Cursor;
use crate::protocol_new::answer::resource::Resource;
use crate::protocol_new::unzip_domain;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CnameResource {
    name: String,
    ttl: u32,
    data: String,
}

impl Resource<String> for CnameResource {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_ttl(&self) -> u32 {
        self.ttl
    }

    fn get_data(&self) -> &String {
        &self.data
    }
}

impl CnameResource {
    pub fn from(name: String, ttl: u32, cursor: &mut Cursor<u8>) -> Self {
        let _data_len = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let data = unzip_domain(cursor);
        CnameResource {
            name,
            ttl,
            data,
        }
    }
}