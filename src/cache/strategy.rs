use crate::cache::DNSCacheRecord;
use crate::protocol::DNSAnswer;
use crate::cache::limit_map::LimitedMap;
use std::sync::Arc;
use crate::system::{Result, get_now, get_sub_now};
use std::time::Duration;

pub trait CacheStrategy: Send + Sync {
    fn handle(&self, key: Vec<u8>, record: DNSCacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer>;
}

pub struct ExpiredCacheStrategy {
    map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>,
}

impl CacheStrategy for ExpiredCacheStrategy {
    fn handle(&self, key: Vec<u8>, record: DNSCacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer> {
        if record.is_expired(get_now()) {
            self.map.remove(&key);
            let answer = get_value_fn()?;
            self.map.insert(key, answer.clone().into());
            Ok(answer)
        } else {
            Ok(record.into())
        }
    }
}

impl ExpiredCacheStrategy {
    pub fn from(map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>) -> Self {
        ExpiredCacheStrategy {
            map
        }
    }
}

pub struct TimeoutCacheStrategy {
    map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>,
    timeout: u128,
}

impl CacheStrategy for TimeoutCacheStrategy {
    fn handle(&self, key: Vec<u8>, record: DNSCacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer> {
        let now = get_sub_now(Duration::from_millis(self.timeout as u64));
        if record.is_expired(now) {
            self.map.remove(&key);
            let answer = get_value_fn()?;
            self.map.insert(key, answer.clone().into());
            Ok(answer)
        } else {
            if record.is_expired(get_now()) {
                let cloned_map = self.map.clone();
                tokio::spawn(async move {
                    match get_value_fn() {
                        Ok(answer) => {
                            cloned_map.insert(key, answer.into());
                        }
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                });
                Ok(record.into())
            } else {
                Ok(record.into())
            }
        }
    }
}

impl TimeoutCacheStrategy {
    pub fn from(map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>, timeout: u128) -> Self {
        TimeoutCacheStrategy {
            map,
            timeout,
        }
    }
}