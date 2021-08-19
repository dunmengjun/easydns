use crate::cursor::Cursor;
use crate::protocol::answer::resource::{Resource, BasicData};
use crate::protocol::unzip_domain;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CnameResource {
    basic: BasicData,
    data: String,
}

impl Resource<String> for CnameResource {
    fn get_name(&self) -> &String {
        self.basic.get_name()
    }

    fn get_ttl(&self) -> u32 {
        self.basic.get_ttl()
    }

    fn get_data(&self) -> &String {
        &self.data
    }
}

impl CnameResource {
    pub fn create(basic: BasicData, cursor: &Cursor<u8>) -> Self {
        let data = unzip_domain(cursor);
        CnameResource {
            basic,
            data,
        }
    }
}