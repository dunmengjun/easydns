use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::header::Header;
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;
use crate::protocol_new::basic::{BasicData, BasicDataBuilder};

pub struct FailureAnswer {
    data: BasicData,
}

impl Display for FailureAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FAILURE, {})", self.data.get_name())
    }
}

impl Answer for FailureAnswer {
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

impl FailureAnswer {
    pub fn from(data: BasicData) -> Self {
        FailureAnswer {
            data
        }
    }

    pub fn new(id: u16, name: String) -> Self {
        let data = BasicDataBuilder::new()
            .id(id)
            .name(name)
            .flags(0x8182)
            .build();
        FailureAnswer {
            data
        }
    }
}