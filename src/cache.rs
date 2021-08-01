use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::protocol::{DNSAnswer, DNSQuery};
use std::cell::RefCell;

pub struct DNSCacheManager {
    records: BTreeMap<String, DNSCacheRecord>,
}

#[derive(Clone)]
pub struct DNSCacheRecord {
    domain: String,
    address: String,
    ttl: u128,
    last_used_time: u128,
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_millis()
}

impl DNSCacheRecord {
    fn from() -> Self {
        DNSCacheRecord {
            domain: "".to_string(),
            address: "".to_string(),
            ttl: 0,
            last_used_time: 0,
        }
    }
    fn is_expired(&self) -> bool {
        (get_timestamp() - self.last_used_time) > self.ttl
    }

    pub fn get_domain(&self) -> &String {
        &self.domain
    }
    pub fn get_address(&self) -> &String {
        &self.address
    }

    pub fn get_ttl(&self) -> &u128 {
        &self.ttl
    }
}

impl DNSCacheManager {
    pub fn new() -> Self {
        DNSCacheManager {
            records: Default::default()
        }
    }
    pub fn store(&mut self, mut records: Vec<DNSCacheRecord>) {
        for record in records {
            self.records.insert(record.domain.clone(), record);
        }
    }
    pub fn get(&self, domain: &String) -> Option<&DNSCacheRecord> {
        self.records.get(domain)
    }
    pub fn remove(&mut self, domain: &String) {
        self.records.remove(domain);
    }
}

pub fn get_answer(cache_manager: &mut DNSCacheManager, query: &DNSQuery) -> Option<DNSAnswer> {
    let mut records = Vec::new();
    let mut expired_domains = Vec::new();
    for domain in query.get_domains() {
        if let Some(r) = cache_manager.get(&domain) {
            if r.is_expired() {
                expired_domains.push(domain)
            } else {
                records.push(r);
            }
        }
    }
    let result = if !records.is_empty() {
        Some(DNSAnswer::from_cache(query.get_id().clone(), records))
    } else {
        None
    };
    for domain in expired_domains {
        cache_manager.remove(&domain);
    }
    result
}

pub fn store_answer(cache_manager: &mut DNSCacheManager, answer: DNSAnswer) {
    cache_manager.store(answer.into());
}