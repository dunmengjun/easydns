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
            last_used_time: get_timestamp(),
        }
    }
    fn is_expired(&self) -> bool {
        (get_timestamp() - self.last_used_time) > self.ttl
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.domain
    }
    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }

    pub fn get_ttl_secs(&self) -> u128 {
        self.ttl / 1000
    }
    pub fn pass_ttl(&mut self) {
        let current_time = get_timestamp();
        self.ttl = self.ttl - (current_time - self.last_used_time);
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
        let mut buf = [0u8; 16];
        for i in 0..16 {
            buf[i] = split[3][i];
        }
        let last_used_time = u128::from_be_bytes(buf);
        DNSCacheRecord {
            domain,
            address,
            ttl,
            last_used_time,
        }
    }
}

impl DNSCacheManager {
    fn from(limit_len: usize) -> Self {
        DNSCacheManager {
            records: Default::default(),
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
    fn get(&self, domain: &Vec<u8>) -> Option<Ref<Vec<u8>, DNSCacheRecord, RandomState>> {
        self.records.remove_if(domain, |_, v| {
            v.is_expired()
        });
        self.records.get_mut(domain).map(|mut e| {
            e.pass_ttl();
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
        let records = DashMap::new();
        for r_bytes in split {
            let record = DNSCacheRecord::from_bytes(r_bytes);
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
}

impl CacheContext {
    async fn from(config: &Config) -> Self {
        if !config.cache_on {
            return CacheContext {
                cache_on: false,
                manager: DNSCacheManager::from(config.cache_num),
            };
        }
        let manager = match File::open(config.cache_file).await {
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

fn cache_on() -> bool {
    CACHE_CONTEXT.get().unwrap().cache_on
}

fn cache_manager() -> &'static DNSCacheManager {
    &CACHE_CONTEXT.get().unwrap().manager
}

pub fn get_answer(query: &DNSQuery) -> Option<DNSAnswer> {
    if !cache_on() {
        return None;
    }
    cache_manager().get(query.get_domain())
        .map(|r|
            DNSAnswer::from_cache(query.get_id().clone(), r.value()))
}

pub fn store_answer(answer: &DNSAnswer) {
    if !cache_on() {
        return;
    }
    cache_manager().store(answer.to_cache())
}

pub async fn run_abort_action() -> Result<()> {
    if !cache_on() {
        info!("缓存已禁用");
        return Ok(());
    }
    if cache_manager().records.is_empty() {
        info!("没有缓存需要写入文件");
        return Ok(());
    }
    let mut file = File::create("cache").await?;
    file.write_all(cache_manager().to_file_bytes().as_slice()).await?;
    info!("缓存全部写入了文件! 文件名称是cache");
    Ok(())
}