use crate::protocol_new::answer::Answer;
use crate::cache::{CacheRecord, SoaCacheRecord, CacheItem};
use crate::protocol_new::answer::resource::{SoaResource, Resource};
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::basic::{BasicData, Builder};
use crate::system::get_now;

pub struct SoaAnswer {
    data: BasicData,
    resource: SoaResource,
}

impl Display for SoaAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(SOA, {}, {})", self.data.get_name(), self.resource.get_ttl())
    }
}

impl From<&SoaCacheRecord> for SoaAnswer {
    fn from(record: &SoaCacheRecord) -> Self {
        let data = Builder::new()
            .name(record.get_key().clone())
            .flags(0x8180)
            .authority(1)
            .build();
        let resource = SoaResource::new_with_default_soa(
            record.get_key().clone(), record.get_remain_time(get_now()) as u32);
        SoaAnswer {
            data,
            resource,
        }
    }
}

impl Answer for SoaAnswer {
    fn to_cache(&self) -> Option<CacheRecord> {
        Some(SoaCacheRecord::from(self).into())
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
    pub fn create(data: BasicData, mut resource: SoaResource) -> Self {
        resource.set_name(data.get_name().clone());
        SoaAnswer {
            data,
            resource,
        }
    }

    pub fn default_soa(id: u16, name: String) -> Self {
        let data = Builder::new()
            .id(id)
            .name(name.clone())
            .flags(0x8180)
            .authority(1)
            .build();
        SoaAnswer {
            data,
            resource: SoaResource::new_with_default_soa(name, 600),
        }
    }

    pub fn get_name(&self) -> &String {
        self.data.get_name()
    }

    pub fn get_ttl(&self) -> u32 {
        self.resource.get_ttl()
    }
}