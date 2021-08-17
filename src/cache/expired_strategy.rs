use crate::cache::{CacheStrategy, CacheMap, AnswerFuture};
use std::sync::Arc;
use crate::protocol::DNSAnswer;
use crate::system::get_now;
use crate::system::Result;
use crate::cache::cache_record::{CacheRecord, Expired};
use async_trait::async_trait;

pub struct ExpiredCacheStrategy {
    map: Arc<CacheMap>,
}

#[async_trait]
impl CacheStrategy for ExpiredCacheStrategy {
    async fn handle(&self, record: CacheRecord, future: AnswerFuture) -> Result<DNSAnswer> {
        if record.is_expired(get_now()) {
            let answer = future.await?;
            if let Some(r) = answer.to_cache() {
                self.map.insert(record.get_key().clone(), r);
            }
            Ok(answer)
        } else {
            Ok(record.to_answer())
        }
    }
}

impl ExpiredCacheStrategy {
    pub fn from(map: Arc<CacheMap>) -> Self {
        ExpiredCacheStrategy {
            map
        }
    }
}

// #[cfg(test)]
// pub mod tests {
//     use crate::cache::expired_strategy::ExpiredCacheStrategy;
//     use crate::cache::limit_map::LimitedMap;
//     use crate::cache::{DNSCacheRecord, CacheStrategy};
//     use std::sync::Arc;
//     use crate::cache::record::tests::{get_valid_record, build_valid_record};
//     use crate::protocol::tests::get_ip_answer;
//     use crate::protocol::DNSAnswer;
//     use crate::system::{Result, TIME};
//     use std::sync::atomic::{AtomicBool, Ordering};
//
//     #[test]
//     fn should_return_answer_when_call_handle_given_no_expired_record() {
//         let strategy = ExpiredCacheStrategy {
//             map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(0))
//         };
//         let (is_called, func) = get_test_func();
//         let record = get_valid_record();
//
//         let result = strategy.handle(record.domain.clone(), record, func);
//
//         assert!(!is_called.load(Ordering::Relaxed));
//         assert!(result.is_ok());
//         assert_eq!(get_ip_answer(), result.unwrap());
//         assert!(strategy.map.is_empty());
//     }
//
//     #[test]
//     fn should_return_answer_and_call_get_data_func_and_insert_map_when_call_handle_given_expired_record() {
//         let strategy = ExpiredCacheStrategy {
//             map: Arc::new(LimitedMap::<Vec<u8>, DNSCacheRecord>::from(10))
//         };
//         let (is_called, func) = get_test_func();
//         let record = get_expired_record();
//         let key = record.domain.clone();
//
//         let result = strategy.handle(key.clone(), record, func);
//
//         assert!(is_called.load(Ordering::Relaxed));
//         assert!(result.is_ok());
//         assert_eq!(get_ip_answer(), result.unwrap());
//         let expected = build_valid_record(|r| { r.start_time = 1001; });
//         assert_eq!(Some(expected), strategy.map.get(&key))
//     }
//
//     pub fn get_test_func() -> (Arc<AtomicBool>, Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) {
//         let is_called = Arc::new(AtomicBool::new(false));
//         let rc = is_called.clone();
//         let func = Box::new(move || -> Result<DNSAnswer>{
//             rc.fetch_or(true, Ordering::Relaxed);
//             Ok(get_ip_answer())
//         });
//         (is_called, func)
//     }
//
//     fn get_expired_record() -> DNSCacheRecord {
//         let record = get_valid_record();
//         TIME.with(|r| {
//             r.borrow_mut().set_timestamp(1001);
//         });
//         record
//     }
// }