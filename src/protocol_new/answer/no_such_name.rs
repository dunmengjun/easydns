use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;
use crate::protocol_new::basic::BasicData;

pub struct NoSuchNameAnswer {
    data: BasicData,
}

impl Display for NoSuchNameAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(NO_SUCH_NAME, {})", self.data.get_name())
    }
}

impl Answer for NoSuchNameAnswer {
    fn to_cache(&self) -> Option<CacheRecord> {
        todo!()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let data = &self.data;
        data.into()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }

    fn set_id(&mut self, id: u16) {
        self.data.set_id(id);
    }

    fn get_id(&self) -> u16 {
        self.data.get_id()
    }
}

impl NoSuchNameAnswer {
    pub fn from(data: BasicData) -> Self {
        NoSuchNameAnswer {
            data
        }
    }
}