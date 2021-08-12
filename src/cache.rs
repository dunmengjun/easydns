use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::{Result, get_timestamp};
use dashmap::DashMap;
use std::collections::hash_map::RandomState;
use dashmap::mapref::one::{Ref};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use crate::config::Config;

const F_DELIMITER: u8 = '|' as u8;
const F_SPACE: u8 = ' ' as u8;

pub struct DNSCacheManager {
    records: DashMap<Vec<u8>, DNSCacheRecord>,
    limit_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DNSCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
    pub ttl: u128,
    pub pass_time: u128,
    pub last_used_time: u128,
}

impl DNSCacheRecord {
    pub fn from(
        domain: Vec<u8>,
        address: Vec<u8>,
        ttl_secs: u32) -> Self {
        DNSCacheRecord {
            domain,
            address,
            ttl: ttl_secs as u128 * 1000 as u128,
            pass_time: 0,
            last_used_time: get_timestamp(),
        }
    }
    fn is_expired(&self) -> bool {
        self.pass_time > self.ttl
    }

    fn get_pass_time(&self) -> u128 {
        self.pass_time
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.domain
    }
    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }

    pub fn get_ttl_secs(&self) -> u128 {
        if self.ttl < self.pass_time {
            0
        } else {
            (self.ttl - self.pass_time) / 1000
        }
    }

    pub fn get_ttl(&self) -> u128 {
        self.ttl
    }
    pub fn pass_time(&mut self) {
        let current_time = get_timestamp();
        let time = current_time - self.last_used_time;
        self.pass_time += time;
        self.last_used_time = current_time;
    }

    fn to_file_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::<u8>::new();
        vec.extend(&self.domain);
        vec.push(F_DELIMITER);
        vec.extend(&self.address);
        vec.push(F_DELIMITER);
        vec.extend(&(self.ttl as u32).to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&(self.pass_time as u32).to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&self.last_used_time.to_be_bytes());
        vec.push(F_SPACE);
        vec
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let split: Vec<&[u8]> = bytes.split(|e| F_DELIMITER == *e).collect();
        let domain = Vec::<u8>::from(split[0]);
        let address = Vec::<u8>::from(split[1]);
        let mut buf = [0u8; 4];
        for i in 0..4 {
            buf[i] = split[2][i]
        }
        let ttl = u32::from_be_bytes(buf) as u128;
        let mut buf = [0u8; 4];
        for i in 0..4 {
            buf[i] = split[3][i]
        }
        let pass_time = u32::from_be_bytes(buf) as u128;
        let mut buf = [0u8; 16];
        for i in 0..16 {
            buf[i] = split[4][i];
        }
        let last_used_time = u128::from_be_bytes(buf);
        DNSCacheRecord {
            domain,
            address,
            ttl,
            pass_time,
            last_used_time,
        }
    }
}

impl DNSCacheManager {
    fn from(limit_len: usize) -> Self {
        DNSCacheManager {
            records: DashMap::with_capacity(limit_len),
            limit_len,
        }
    }
    fn store(&self, record: DNSCacheRecord) {
        //如果缓存超过了限制的大小，则删除掉十分之一的快过期的记录
        if self.records.len() >= self.limit_len {
            let vec = &mut Vec::new();
            self.records.iter().for_each(|e| {
                vec.push(e)
            });
            vec.sort_unstable_by_key(|e| e.ttl);
            vec[0..self.limit_len / 10].iter().for_each(|e| {
                self.records.remove(e.key());
            });
        }
        self.records.insert(record.domain.clone(), record);
    }
    fn get_or_remove(&self, domain: &Vec<u8>, remove_if: impl FnOnce(&DNSCacheRecord) -> bool)
                     -> Option<Ref<Vec<u8>, DNSCacheRecord, RandomState>> {
        self.records.remove_if(domain, |_, v| {
            remove_if(v)
        });
        self.records.get_mut(domain).map(|mut e| {
            e.pass_time();
            e.downgrade()
        })
    }
    fn to_file_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.records.iter().for_each(|e| {
            vec.extend(e.value().to_file_bytes());
        });
        vec.remove(vec.len() - 1);
        vec
    }

    fn from_bytes(bytes: &[u8], limit_len: usize) -> Self {
        let split = bytes.split(|e| F_SPACE == *e);
        let records = DashMap::with_capacity(limit_len);
        for r_bytes in split {
            let mut record = DNSCacheRecord::from_bytes(r_bytes);
            record.pass_time();
            if !record.is_expired() {
                records.insert(record.domain.clone(), record);
            }
        }
        DNSCacheManager {
            records,
            limit_len,
        }
    }
}

pub struct CachePool {
    cache_on: bool,
    manager: DNSCacheManager,
    cache_get_strategy: usize,
    cache_ttl_timeout_ms: usize,
    cache_file: String,
}

impl CachePool {
    fn new(config: &Config) -> Self {
        CachePool {
            cache_on: false,
            manager: DNSCacheManager::from(config.cache_num),
            cache_get_strategy: config.cache_get_strategy,
            cache_ttl_timeout_ms: config.cache_ttl_timeout_ms,
            cache_file: config.cache_file.clone(),
        }
    }
    pub async fn from(config: &Config) -> Self {
        if !config.cache_on {
            return CachePool::new(config);
        }
        let manager = match File::open(&config.cache_file).await {
            Ok(mut file) => {
                let file_vec = &mut Vec::new();
                file.read_to_end(file_vec).await.unwrap();
                if file_vec.is_empty() {
                    DNSCacheManager::from(config.cache_num)
                } else {
                    DNSCacheManager::from_bytes(file_vec.as_slice(), config.cache_num)
                }
            }
            Err(_e) => {
                DNSCacheManager::from(config.cache_num)
            }
        };
        CachePool {
            cache_on: config.cache_on,
            manager,
            cache_get_strategy: config.cache_get_strategy,
            cache_ttl_timeout_ms: config.cache_ttl_timeout_ms,
            cache_file: config.cache_file.clone(),
        }
    }

    fn is_should_removed(&self) -> impl FnOnce(&DNSCacheRecord) -> bool + '_ {
        move |r: &DNSCacheRecord| {
            if self.cache_get_strategy == 1 {
                r.is_expired() && r.get_pass_time() > r.get_ttl() + self.cache_ttl_timeout_ms as u128
            } else {
                r.is_expired()
            }
        }
    }

    pub fn get_answer<F>(&self, query: &DNSQuery, async_func: F) -> Option<DNSAnswer> where F: FnOnce() {
        if !self.cache_on {
            return None;
        }
        self.manager.get_or_remove(query.get_domain(), self.is_should_removed())
            .map(|r| {
                if self.cache_get_strategy == 1 && r.is_expired() {
                    async_func()
                }
                DNSAnswer::from_cache(query.get_id().clone(), r.value())
            })
    }

    pub fn store_answer(&self, answer: &DNSAnswer) {
        if !self.cache_on {
            return;
        }
        self.manager.store(answer.to_cache())
    }

    pub async fn exit_process_action(&self) -> Result<()> {
        if !self.cache_on {
            info!("缓存已禁用");
            return Ok(());
        }
        if self.manager.records.is_empty() {
            info!("没有缓存需要写入文件");
            return Ok(());
        }
        let mut file = File::create(&self.cache_file).await?;
        file.write_all(self.manager.to_file_bytes().as_slice()).await?;
        info!("缓存全部写入了文件! 文件名称是cache");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::{DNSCacheRecord, CachePool};
    use crate::protocol::{DNSQuery};
    use crate::config::Config;
    use crate::system::Result;
    use crate::protocol::tests::build_simple_answer;
    use crate::config::tests::init_test_config;

    #[tokio::test]
    async fn should_return_none_when_get_answer_given_cache_on_is_false() -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = false;
        }).await;
        let query = DNSQuery::from_domain("www.baidu.com");

        let result = cache_pool.get_answer(&query, || assert!(false));

        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn should_return_answer_when_get_answer_given_cache_is_enable_and_has_cache() -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = true;
            config.cache_get_strategy = 0;
        }).await;
        let query = init_data(&cache_pool, |_r| {});

        let mut result = cache_pool.get_answer(&query, || assert!(false));

        result.iter_mut().for_each(|e| e.set_all_ttl(0));
        let expected = build_simple_answer(&query, vec![1, 1, 1, 1], 0);
        assert_eq!(Some(expected), result);
        Ok(())
    }

    #[tokio::test]
    async fn should_return_none_when_get_answer_given_record_is_expired() -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = true;
            config.cache_get_strategy = 0;
        }).await;
        let query = init_data(&cache_pool, |record| {
            record.ttl = 1000;
            record.pass_time = 1001;
        });

        let result = cache_pool.get_answer(&query, || assert!(false));

        assert!(cache_pool.manager.records.is_empty());
        assert_eq!(None, result);
        Ok(())
    }

    #[tokio::test]
    async fn should_return_answer_and_call_async_method_when_get_answer_given_cache_strategy_is_1_and_record_is_in_timeout()
        -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = true;
            config.cache_get_strategy = 1;
            config.cache_ttl_timeout_ms = 1000;
        }).await;
        let query = init_data(&cache_pool, |record| {
            record.ttl = 1000;
            record.pass_time = 1001;
        });
        let mut async_call = false;

        let result = cache_pool.get_answer(&query, || { async_call = true; });

        let expected = build_simple_answer(&query, vec![1, 1, 1, 1], 0);
        assert!(async_call);
        assert_eq!(Some(expected), result);
        Ok(())
    }

    #[tokio::test]
    async fn should_return_none_when_get_answer_given_cache_strategy_is_1_and_record_is_over_timeout()
        -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = true;
            config.cache_get_strategy = 1;
            config.cache_ttl_timeout_ms = 1000;
        }).await;
        let query = init_data(&cache_pool, |record| {
            record.ttl = 1000;
            record.pass_time = 2001;
        });

        let result = cache_pool.get_answer(&query, || assert!(false));

        assert!(cache_pool.manager.records.is_empty());
        assert_eq!(None, result);
        Ok(())
    }

    #[tokio::test]
    async fn should_no_record_in_context_when_store_answer_given_cache_on_is_false() -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = false;
        }).await;
        let query = DNSQuery::from_domain("www.baidu.com");
        let answer = build_simple_answer(&query, vec![1, 1, 1, 1], 1);

        cache_pool.store_answer(&answer);

        assert!(cache_pool.manager.records.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn should_store_record_in_context_when_store_answer_given_cache_on_is_true() -> Result<()> {
        let cache_pool = init_cache_pool(|config| {
            config.cache_on = true;
        }).await;
        let query = DNSQuery::from_domain("www.baidu.com");
        let answer = build_simple_answer(&query, vec![1, 1, 1, 1], 1);

        cache_pool.store_answer(&answer);

        assert_eq!(1, cache_pool.manager.records.len());
        let expected = init_record(query.get_domain().clone());
        let result = cache_pool.manager.records.get(query.get_domain()).unwrap().eq(&expected);
        assert!(result);
        Ok(())
    }

    async fn init_cache_pool(f: impl Fn(&mut Config)) -> CachePool {
        let mut config = init_test_config();
        f(&mut config);
        CachePool::from(&config).await
    }

    fn init_data(cache_pool: &CachePool, f: impl Fn(&mut DNSCacheRecord)) -> DNSQuery {
        let query = DNSQuery::from_domain("www.baidu.com");
        let mut record = init_record(query.get_domain().clone());
        f(&mut record);
        cache_pool.manager.records.insert(record.domain.clone(), record.clone());
        query
    }

    fn init_record(domain: Vec<u8>) -> DNSCacheRecord {
        DNSCacheRecord::from(domain, vec![1, 1, 1, 1], 1)
    }
}