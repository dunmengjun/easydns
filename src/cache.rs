use crate::protocol::{DNSAnswer, DNSQuery};
use crate::timer::get_timestamp;
use crate::system::{AbortFunc, Result};
use once_cell::sync::Lazy;
use dashmap::DashMap;
use std::collections::hash_map::RandomState;
use dashmap::mapref::one::{Ref};

pub struct DNSCacheManager {
    records: DashMap<Vec<u8>, DNSCacheRecord>,
}

pub struct DNSCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
    pub ttl: u128,
    pub last_used_time: u128,
}

impl DNSCacheRecord {
    pub fn from(
        domain: Vec<u8>,
        address: Vec<u8>,
        ttl: u32) -> Self {
        DNSCacheRecord {
            domain,
            address,
            ttl: ttl as u128 * 1000 as u128,
            last_used_time: get_timestamp(),
        }
    }
    fn is_expired(&self) -> bool {
        (get_timestamp() - self.last_used_time) > self.ttl
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.domain
    }
    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }

    pub fn get_ttl_secs(&self) -> u128 {
        self.ttl / 1000
    }
    pub fn pass_ttl(&mut self) {
        let current_time = get_timestamp();
        self.ttl = self.ttl - (current_time - self.last_used_time);
        self.last_used_time = current_time;
    }
}

impl DNSCacheManager {
    pub fn new() -> Self {
        DNSCacheManager {
            records: Default::default(),
        }
    }
    pub fn store(&self, record: DNSCacheRecord) {
        self.records.insert(record.domain.clone(), record);
    }
    pub fn get(&self, domain: &Vec<u8>) -> Option<Ref<Vec<u8>, DNSCacheRecord, RandomState>> {
        self.records.remove_if(domain, |_, v| {
            v.is_expired()
        });
        self.records.get_mut(domain).map(|mut e| {
            e.pass_ttl();
            e.downgrade()
        })
    }
}

static CACHE_MANAGER: Lazy<DNSCacheManager> = Lazy::new(|| {
    DNSCacheManager::new()
});

pub fn get_answer(query: &DNSQuery) -> Result<Option<DNSAnswer>> {
    Ok(CACHE_MANAGER.get(query.get_domain())
        .map(|r|
            DNSAnswer::from_cache(query.get_id().clone(), r.value())))
}

pub fn store_answer(answer: DNSAnswer) -> Result<()> {
    Ok(CACHE_MANAGER.store(answer.into()))
}

pub fn get_abort_action() -> AbortFunc {
    Box::new(move || {
        println!("缓存abort处理成功!");
    })
}