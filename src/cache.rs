use std::collections::BTreeMap;
use crate::protocol::{DNSAnswer, DNSQuery};
use std::sync::{RwLock};
use crate::timer::get_timestamp;
use crate::error::Result;

pub struct DNSCacheManager {
    inner: RwLock<DnsCacheInner>,
}

pub struct DnsCacheInner {
    records: BTreeMap<Vec<u8>, DNSCacheRecord>,
}

pub struct DNSCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
    pub ttl: u128,
    pub last_used_time: u128,
}

impl DNSCacheRecord {
    fn is_expired(&self) -> bool {
        (get_timestamp() - self.last_used_time) > self.ttl * 1000
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.domain
    }
    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }

    pub fn get_ttl(&self) -> &u128 {
        &self.ttl
    }
}

impl DnsCacheInner {
    pub fn store(&mut self, record: DNSCacheRecord) {
        self.records.insert(record.domain.clone(), record);
    }
    pub fn get(&mut self, domain: &Vec<u8>) -> Option<&DNSCacheRecord> {
        if let Some(r) = self.records.get_mut(domain) {
            if r.is_expired() {
                self.records.remove(domain);
            } else {
                r.last_used_time = get_timestamp();
            }
        }
        self.records.get(domain)
    }
}

unsafe impl Sync for DNSCacheManager {}

impl DNSCacheManager {
    pub fn new() -> Self {
        DNSCacheManager {
            inner: RwLock::new(DnsCacheInner { records: Default::default() }),
        }
    }
}

lazy_static! {
    static ref CACHE_MANAGER: DNSCacheManager = DNSCacheManager::new();
}

pub fn get_answer(query: &DNSQuery) -> Result<Option<DNSAnswer>> {
    Ok(CACHE_MANAGER.inner.write()?.get(query.get_domain())
        .map(|r| DNSAnswer::from_cache(query.get_id().clone(), r)))
}

pub fn store_answer(answer: DNSAnswer) -> Result<()> {
    Ok(CACHE_MANAGER.inner.write()?.store(answer.into()))
}