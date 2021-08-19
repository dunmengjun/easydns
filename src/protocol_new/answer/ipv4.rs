use crate::protocol_new::answer::Answer;
use crate::cache::{CacheRecord, IpCacheRecord, CacheItem};
use crate::protocol_new::answer::resource::{Ipv4Resource, Resource};
use std::fmt::{Display, Formatter};
use std::any::Any;
use crate::protocol_new::{DnsAnswer};
use std::net::Ipv4Addr;
use crate::protocol_new::basic::{BasicData, Builder};

pub struct Ipv4Answer {
    data: BasicData,
    resources: Vec<Ipv4Resource>,
}

impl Display for Ipv4Answer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(IP, {}, {}, {})", self.data.get_name(),
               self.resources[0].get_ttl(), self.resources[0].get_data())
    }
}

impl Answer for Ipv4Answer {
    fn to_cache(&self) -> Option<CacheRecord> {
        Some(IpCacheRecord::from(self).into())
    }

    fn to_bytes(&self) -> Vec<u8> {
        let data = &self.data;
        let mut vec: Vec<u8> = data.into();
        self.resources.iter().for_each(|r| {
            let resource: Vec<u8> = r.into();
            vec.extend(resource)
        });
        vec
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }

    fn set_id(&mut self, id: u16) {
        self.data.set_id(id)
    }

    fn get_id(&self) -> u16 {
        self.data.get_id()
    }
}

impl Ipv4Answer {
    pub fn create(mut data: BasicData, resources: Vec<Ipv4Resource>) -> Self {
        data.set_authority_count(0);
        Ipv4Answer {
            data,
            resources,
        }
    }

    pub fn combine(&mut self, mut other: DnsAnswer) {
        if let Some(answer) = other.as_mut_any().downcast_mut::<Self>() {
            if self.get_name() != answer.get_name() {
                return;
            }
            while let Some(r) = answer.resources.pop() {
                let flag = self.resources.iter().find(|e| {
                    e.data != r.data
                }).is_none();
                if flag {
                    self.resources.push(r);
                }
            }
            self.data.set_answer_count(self.resources.len() as u16);
        }
    }
    pub fn empty_answer(id: u16, name: String) -> Self {
        let data = Builder::new()
            .id(id)
            .name(name)
            .flags(0x8180)
            .build();
        Ipv4Answer {
            data,
            resources: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    pub fn get_all_ips(&self) -> Vec<&Ipv4Addr> {
        self.resources.iter().map(|r| {
            &r.data
        }).collect()
    }

    pub fn retain_ip(&mut self, ip: &Ipv4Addr) {
        self.resources.retain(|r| {
            r.data.eq(ip)
        });
        self.data.set_answer_count(1);
    }

    pub fn get_name(&self) -> &String {
        self.data.get_name()
    }

    pub fn get_ttl(&self) -> u32 {
        self.resources[0].get_ttl()
    }

    pub fn get_address(&self) -> &Ipv4Addr {
        self.resources[0].get_data()
    }
}

impl From<&IpCacheRecord> for Ipv4Answer {
    fn from(record: &IpCacheRecord) -> Self {
        let data = Builder::new()
            .flags(0x8180)
            .name(record.get_key().clone())
            .answer(1)
            .build();
        let resource = Ipv4Resource::from(record);
        Ipv4Answer {
            data,
            resources: vec![resource],
        }
    }
}