mod ip_record;
mod soa_record;

use crate::system::get_now;
use crate::cache::limit_map::GetOrdKey;

pub use ip_record::IpCacheRecord;
pub use soa_record::SoaCacheRecord;
use std::fmt::{Debug, Formatter};
use crate::protocol_new::DnsAnswer;

pub type CacheRecord = Box<dyn CacheItem>;

pub const IP_RECORD: u8 = '*' as u8;
pub const SOA_RECORD: u8 = '#' as u8;

pub trait Expired {
    fn is_expired(&self, timestamp: u128) -> bool;
}

impl Expired for CacheRecord {
    fn is_expired(&self, timestamp: u128) -> bool {
        let duration = timestamp - self.get_create_time();
        self.get_ttl_ms() < duration
    }
}

pub trait CacheItem: Sync + Send + BoxedClone {
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
    fn to_bytes(&self) -> Vec<u8>;
    fn to_answer(&self) -> DnsAnswer;
}

pub trait BoxedClone {
    fn boxed_clone(&self) -> CacheRecord;
}

impl<T> BoxedClone for T where T: 'static + Clone + CacheItem {
    fn boxed_clone(&self) -> CacheRecord {
        Box::new(self.clone())
    }
}

impl Clone for CacheRecord {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl GetOrdKey for CacheRecord {
    type Output = u128;
    fn get_order_key(&self) -> Self::Output {
        self.get_remain_time(get_now())
    }
}

impl Debug for CacheRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("")
            .field(self.get_key())
            .field(&self.get_create_time())
            .field(&self.get_remain_time(get_now()))
            .field(&self.get_ttl_ms())
            .finish()
    }
}

impl From<IpCacheRecord> for CacheRecord {
    fn from(record: IpCacheRecord) -> Self {
        Box::new(record)
    }
}

impl From<SoaCacheRecord> for CacheRecord {
    fn from(record: SoaCacheRecord) -> Self {
        Box::new(record)
    }
}

#[cfg(test)]
impl PartialEq for CacheRecord {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes().eq(&other.to_bytes())
    }
}

#[cfg(test)]
pub mod tests {
    pub use crate::cache::cache_record::ip_record::tests;
    use crate::cache::{CacheItem, CacheRecord};
    use crate::cache::cache_record::BoxedClone;
    use crate::system::TIME;
    use crate::cache::limit_map::GetOrdKey;
    use crate::protocol_new::{DnsAnswer, FailureAnswer};

    #[test]
    fn should_return_true_when_check_expired_given_expired() {
        let record = get_test_record();

        let result = record.is_expired(1001);

        assert!(result)
    }

    #[test]
    fn should_return_false_when_check_expired_given_not_expired() {
        let record = get_test_record();

        let result = record.is_expired(999);

        assert!(!result)
    }

    #[test]
    fn should_return_remain_time_when_get_remain_time_given_not_expired() {
        let record = get_test_record();

        let result = record.get_remain_time(999);

        assert_eq!(1, result)
    }

    #[test]
    fn should_return_0_when_get_remain_time_given_expired() {
        let record = get_test_record();

        let result = record.get_remain_time(1001);

        assert_eq!(0, result)
    }

    #[test]
    fn should_return_remain_time_when_get_order_key_given_test_record() {
        let record: CacheRecord = Box::new(get_test_record());
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(999);
        });

        let result: u128 = record.get_order_key();

        assert_eq!(1, result)
    }

    fn get_test_record() -> TestRecord {
        TestRecord {
            key: vec![],
            ttl: 1000,
            create_time: 0,
        }
    }

    #[derive(Clone)]
    struct TestRecord {
        key: Vec<u8>,
        ttl: u128,
        create_time: u128,
    }

    impl CacheItem for TestRecord {
        fn get_create_time(&self) -> u128 {
            self.create_time
        }

        fn get_ttl_ms(&self) -> u128 {
            self.ttl
        }

        fn get_key(&self) -> &Vec<u8> {
            &self.key
        }

        fn to_bytes(&self) -> Vec<u8> {
            vec![]
        }

        fn to_answer(&self) -> DnsAnswer {
            DnsAnswer::from(FailureAnswer::new(0, "".to_string()))
        }
    }
}
