use crate::cache::limit_map::LimitedMap;
use std::sync::Arc;
use crate::cache::{CacheStrategy};
use crate::protocol::DNSAnswer;
use crate::system::{get_sub_now, get_now};
use std::time::Duration;
use crate::system::Result;
use crate::cache::cache_record::{CacheRecord};

pub struct TimeoutCacheStrategy {
    map: Arc<LimitedMap<Vec<u8>, CacheRecord>>,
    timeout: u128,
}

impl CacheStrategy for TimeoutCacheStrategy {
    fn handle(&self, key: Vec<u8>, record: CacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer> {
        let now = get_sub_now(Duration::from_millis(self.timeout as u64));
        if record.is_expired(now) {
            let answer = get_value_fn()?;
            self.map.insert(key, answer.clone().into());
            Ok(answer)
        } else {
            if record.is_expired(get_now()) {
                let cloned_map = self.map.clone();
                let _joiner = tokio::spawn(async move {
                    match get_value_fn() {
                        Ok(answer) => {
                            cloned_map.insert(key, answer.into());
                        }
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                });
                if cfg!(test) {
                    crate::system::block_on(async move {
                        _joiner.await.unwrap();
                    })
                }
                Ok(record.into())
            } else {
                Ok(record.into())
            }
        }
    }
}

impl TimeoutCacheStrategy {
    pub fn from(map: Arc<LimitedMap<Vec<u8>, CacheRecord>>, timeout: u128) -> Self {
        TimeoutCacheStrategy {
            map,
            timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::timeout_strategy::TimeoutCacheStrategy;
    use std::sync::Arc;
    use crate::cache::limit_map::LimitedMap;
    use crate::cache::{CacheStrategy, CacheRecord, CacheItem};
    use std::sync::atomic::Ordering;
    use crate::protocol::tests::{get_valid_answer, get_valid_answer_with_ttl};
    use crate::cache::expired_strategy::tests::get_test_func;
    use crate::system::{set_time_base};
    use crate::cache::cache_record::tests::tests::{get_valid_record, build_valid_record};

    #[test]
    fn should_return_answer_when_call_handle_given_no_expired_record() {
        let strategy = TimeoutCacheStrategy {
            map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
            timeout: 800,
        };
        let (is_called, func) = get_test_func();
        let record = Box::new(get_valid_record());
        set_time_base(999);

        let result = strategy.handle(record.domain.clone(), record, func);

        assert!(!is_called.load(Ordering::Relaxed));
        assert!(result.is_ok());
        assert_eq!(get_valid_answer_with_ttl(0), result.unwrap());
        assert!(strategy.map.is_empty());
    }

    #[test]
    fn should_return_answer_and_insert_to_map_when_call_handle_given_expired_record() {
        let strategy = TimeoutCacheStrategy {
            map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
            timeout: 1000,
        };
        let (is_called, func) = get_test_func();
        let record = Box::new(get_valid_record());
        set_time_base(2001);
        let key = record.get_key().clone();

        let result = strategy.handle(key.clone(), record, func);

        assert!(is_called.load(Ordering::Relaxed));
        assert!(result.is_ok());
        assert_eq!(get_valid_answer(), result.unwrap());
        let expected = build_valid_record(|r| { r.create_time = 2001; });
        assert_eq!(Some(Box::new(expected)), strategy.map.get(&key));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_answer_and_insert_to_map_when_call_handle_given_no_expired_but_timeout_record() {
        //没找到好的办法测试内部的async调用，所以只能这样了
        let strategy = TimeoutCacheStrategy {
            map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
            timeout: 1000,
        };
        let (is_called, func) = get_test_func();
        let record = Box::new(get_valid_record());
        set_time_base(1999);
        let key = record.get_key().clone();

        let result = strategy.handle(key.clone(), record, func);

        assert!(is_called.load(Ordering::Relaxed));
        assert!(result.is_ok());
        assert_eq!(get_valid_answer_with_ttl(0), result.unwrap());
        let expected: CacheRecord = Box::new(get_valid_record());
        assert!(strategy.map.get(&key).unwrap().value().eq(&expected))
        // assert_eq!(Some(expected), );
    }
}