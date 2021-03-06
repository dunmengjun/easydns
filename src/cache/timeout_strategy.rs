use std::sync::Arc;
use crate::cache::{CacheStrategy, CacheMap, AnswerFuture};
use crate::system::{get_sub_now, get_now};
use std::time::Duration;
use crate::system::Result;
use crate::cache::cache_record::{CacheRecord, Expired};
use async_trait::async_trait;
use crate::protocol::DnsAnswer;

pub struct TimeoutCacheStrategy {
    map: Arc<CacheMap>,
    timeout: u128,
}

#[async_trait]
impl CacheStrategy for TimeoutCacheStrategy {
    async fn handle(&self, record: CacheRecord, future: AnswerFuture) -> Result<DnsAnswer> {
        let now = get_sub_now(Duration::from_millis(self.timeout as u64));
        if record.is_expired(now) {
            let answer = future.await?;
            if let Some(r) = answer.to_cache() {
                self.map.insert(record.get_key().clone(), r);
            }
            Ok(answer)
        } else {
            if record.is_expired(get_now()) {
                let cloned_map = self.map.clone();
                let key = record.get_key().clone();
                let _joiner = tokio::spawn(async move {
                    match future.await {
                        Ok(answer) => {
                            if let Some(r) = answer.to_cache() {
                                cloned_map.insert(key, r);
                            }
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
                Ok(record.to_answer())
            } else {
                Ok(record.to_answer())
            }
        }
    }
}

impl TimeoutCacheStrategy {
    pub fn from(map: Arc<CacheMap>, timeout: u128) -> Self {
        TimeoutCacheStrategy {
            map,
            timeout,
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::cache::timeout_strategy::TimeoutCacheStrategy;
//     use std::sync::Arc;
//     use crate::cache::limit_map::LimitedMap;
//     use crate::cache::{CacheStrategy, CacheRecord, CacheItem};
//     use std::sync::atomic::Ordering;
//     use crate::protocol::tests::{get_ip_answer, get_ip_answer_with_ttl};
//     use crate::cache::expired_strategy::tests::get_test_func;
//     use crate::system::{set_time_base};
//     use crate::cache::cache_record::tests::tests::{get_ip_record, build_ip_record};
//
//     #[test]
//     fn should_return_answer_when_call_handle_given_no_expired_record() {
//         let strategy = TimeoutCacheStrategy {
//             map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
//             timeout: 800,
//         };
//         let (is_called, func) = get_test_func();
//         let record = Box::new(get_ip_record());
//         set_time_base(999);
//
//         let result = strategy.handle(record.domain.clone(), record, func);
//
//         assert!(!is_called.load(Ordering::Relaxed));
//         assert!(result.is_ok());
//         assert_eq!(get_ip_answer_with_ttl(0), result.unwrap());
//         assert!(strategy.map.is_empty());
//     }
//
//     #[test]
//     fn should_return_answer_and_insert_to_map_when_call_handle_given_expired_record() {
//         let strategy = TimeoutCacheStrategy {
//             map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
//             timeout: 1000,
//         };
//         let (is_called, func) = get_test_func();
//         let record = Box::new(get_ip_record());
//         set_time_base(2001);
//         let key = record.get_key().clone();
//
//         let result = strategy.handle(key.clone(), record, func);
//
//         assert!(is_called.load(Ordering::Relaxed));
//         assert!(result.is_ok());
//         assert_eq!(get_ip_answer(), result.unwrap());
//         let expected = build_ip_record(|r| { r.create_time = 2001; });
//         assert_eq!(Some(Box::new(expected)), strategy.map.get(&key));
//     }
//
//     #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
//     async fn should_return_answer_and_insert_to_map_when_call_handle_given_no_expired_but_timeout_record() {
//         //????????????????????????????????????async??????????????????????????????
//         let strategy = TimeoutCacheStrategy {
//             map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0)),
//             timeout: 1000,
//         };
//         let (is_called, func) = get_test_func();
//         let record = Box::new(get_ip_record());
//         set_time_base(1999);
//         let key = record.get_key().clone();
//
//         let result = strategy.handle(key.clone(), record, func);
//
//         assert!(is_called.load(Ordering::Relaxed));
//         assert!(result.is_ok());
//         assert_eq!(get_ip_answer_with_ttl(0), result.unwrap());
//         let expected: CacheRecord = Box::new(get_ip_record());
//         assert!(strategy.map.get(&key).unwrap().value().eq(&expected))
//         // assert_eq!(Some(expected), );
//     }
// }