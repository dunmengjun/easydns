use crate::protocol::{DNSAnswer, DNSQuery};
use crate::system::{Result, get_timestamp};
use dashmap::DashMap;
use std::collections::hash_map::RandomState;
use dashmap::mapref::one::{Ref};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use crate::config::Config;
use tokio::sync::{OnceCell};

const F_DELIMITER: u8 = '|' as u8;
const F_SPACE: u8 = ' ' as u8;

pub struct DNSCacheManager {
    records: DashMap<Vec<u8>, DNSCacheRecord>,
    limit_len: usize,
}

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
        ttl: u32) -> Self {
        DNSCacheRecord {
            domain,
            address,
            ttl: ttl as u128 * 1000 as u128,
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

struct CacheContext {
    cache_on: bool,
    manager: DNSCacheManager,
    cache_get_strategy: usize,
    cache_ttl_timeout_ms: usize,
    cache_file: String,
}

impl CacheContext {
    fn new(config: &Config) -> Self {
        CacheContext {
            cache_on: false,
            manager: DNSCacheManager::from(config.cache_num),
            cache_get_strategy: config.cache_get_strategy,
            cache_ttl_timeout_ms: config.cache_ttl_timeout_ms,
            cache_file: config.cache_file.clone(),
        }
    }
    async fn from(config: &Config) -> Self {
        if !config.cache_on {
            return CacheContext::new(config);
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
        CacheContext {
            cache_on: config.cache_on,
            manager,
            cache_get_strategy: config.cache_get_strategy,
            cache_ttl_timeout_ms: config.cache_ttl_timeout_ms,
            cache_file: config.cache_file.clone(),
        }
    }
}

static CACHE_CONTEXT: OnceCell<CacheContext> = OnceCell::const_new();

pub async fn init_context(config: &Config) -> Result<()> {
    let context = CacheContext::from(config).await;
    match CACHE_CONTEXT.set(context) {
        Ok(_) => {}
        Err(e) => {
            panic!("{}", e);
        }
    }
    Ok(())
}

fn context() -> &'static CacheContext {
    &CACHE_CONTEXT.get().unwrap()
}

fn is_should_removed(r: &DNSCacheRecord) -> bool {
    if context().cache_get_strategy == 1 {
        r.is_expired() && r.get_pass_time() > r.get_ttl() + context().cache_ttl_timeout_ms as u128
    } else {
        r.is_expired()
    }
}

pub fn get_answer<F>(query: &DNSQuery, async_func: F) -> Option<DNSAnswer> where F: Fn(DNSQuery) {
    if !context().cache_on {
        return None;
    }
    context().manager.get_or_remove(query.get_domain(), is_should_removed)
        .map(|r| {
            if context().cache_get_strategy == 1 && r.is_expired() {
                async_func(query.clone())
            }
            DNSAnswer::from_cache(query.get_id().clone(), r.value())
        })
}

pub fn store_answer(answer: &DNSAnswer) {
    if !context().cache_on {
        return;
    }
    context().manager.store(answer.to_cache())
}

pub async fn run_abort_action() -> Result<()> {
    if !context().cache_on {
        info!("缓存已禁用");
        return Ok(());
    }
    if context().manager.records.is_empty() {
        info!("没有缓存需要写入文件");
        return Ok(());
    }
    let mut file = File::create(&context().cache_file).await?;
    file.write_all(context().manager.to_file_bytes().as_slice()).await?;
    info!("缓存全部写入了文件! 文件名称是cache");
    Ok(())
}