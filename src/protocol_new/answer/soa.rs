use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::protocol_new::answer::Answer;
use crate::cache::CacheRecord;
use crate::protocol_new::answer::resource::{SoaResource, Resource};
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::DnsAnswer;
use crate::protocol_new::basic::{BasicData, BasicDataBuilder};

pub struct SoaAnswer {
    data: BasicData,
    resource: SoaResource,
}

impl Display for SoaAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(SOA, {}, {})", self.data.get_name(), self.resource.get_ttl())
    }
}

impl Answer for SoaAnswer {
    fn to_cache(&self) -> Option<CacheRecord> {
        todo!()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let data = &self.data;
        let resource1 = &self.resource;
        let mut vec: Vec<u8> = data.into();
        let resource: Vec<u8> = resource1.into();
        vec.extend(resource);
        vec
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

impl SoaAnswer {
    pub fn from(data: BasicData, resource: SoaResource) -> Self {
        SoaAnswer {
            data,
            resource,
        }
    }

    pub fn default_soa(id: u16, name: String) -> Self {
        let data = BasicDataBuilder::new()
            .id(id)
            .name(name.clone())
            .flags(0x8180)
            .authority(1)
            .build();
        SoaAnswer {
            data,
            resource: SoaResource::new_wit_default_soa(name, 600),
        }
    }
}