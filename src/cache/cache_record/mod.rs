mod ip_record;
mod soa_record;

use crate::system::get_now;
use crate::protocol::DNSAnswer;
use crate::cache::limit_map::GetOrdKey;

pub use ip_record::IpCacheRecord;
pub use soa_record::SoaCacheRecord;

pub type CacheRecord = Box<dyn CacheItem>;

pub const IP_RECORD: u8 = '*' as u8;
pub const SOA_RECORD: u8 = '#' as u8;

pub trait CacheItem: Sync + Send + BoxedClone {
    fn is_expired(&self, timestamp: u128) -> bool {
        let duration = timestamp - self.get_create_time();
        self.get_ttl_ms() < duration
    }
    fn get_remain_time(&self, timestamp: u128) -> u128 {
        let duration = timestamp - self.get_create_time();
        if self.get_ttl_ms() > duration {
            self.get_ttl_ms() - duration
        } else {
            0
        }
    }
    fn get_create_time(&self) -> u128;
    fn get_ttl_ms(&self) -> u128;
    fn get_key(&self) -> &Vec<u8>;
}

pub trait BoxedClone {
    fn boxed_clone(&self) -> CacheRecord;
}

impl<T> BoxedClone for T where T: 'static + Clone + CacheItem {
    fn boxed_clone(&self) -> CacheRecord {
        Box::new(self.clone())
    }
}

impl GetOrdKey for CacheRecord {
    type Output = u128;
    fn get_order_key(&self) -> Self::Output {
        self.get_remain_time(get_now())
    }
}

impl From<DNSAnswer> for CacheRecord {
    fn from(answer: DNSAnswer) -> Self {
        if !answer.is_empty_answers() {
            Box::new(IpCacheRecord::from(answer))
        } else {
            Box::new(SoaCacheRecord::from(answer))
        }
    }
}

impl From<CacheRecord> for DNSAnswer {
    fn from(r: CacheRecord) -> Self {
        r.into()
    }
}

impl From<CacheRecord> for Vec<u8> {
    fn from(r: CacheRecord) -> Self {
        r.into()
    }
}

#[cfg(test)]
pub mod tests {
    pub use crate::cache::cache_record::ip_record::tests;
}
