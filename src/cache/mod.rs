mod limit_map;
mod expired_strategy;
mod timeout_strategy;
mod cache_record;

use crate::protocol::DNSAnswer;
use crate::config::Config;
use crate::system::{Result, get_now, block_on};
use std::sync::Arc;
use limit_map::{LimitedMap};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub use cache_record::CacheRecord;
pub use cache_record::IpCacheRecord;
pub use cache_record::SoaCacheRecord;
pub use cache_record::CacheItem;
use crate::cache::expired_strategy::ExpiredCacheStrategy;
use crate::cache::timeout_strategy::TimeoutCacheStrategy;
use crate::cache::cache_record::IP_RECORD;
use crate::cache::cache_record::SOA_RECORD;
use crate::cursor::Cursor;

pub type GetAnswerFunc = Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>;
pub type CacheMap = LimitedMap<Vec<u8>, CacheRecord>;
type ExpiredStrategy = Box<dyn CacheStrategy>;

pub trait CacheStrategy: Send + Sync {
    fn handle(&self, record: CacheRecord, get_value_fn: GetAnswerFunc) -> Result<DNSAnswer>;
}

pub struct CachePool {
    strategy: ExpiredStrategy,
    file_name: String,
    map: Arc<CacheMap>,
}

impl Drop for CachePool {
    fn drop(&mut self) {
        block_on(async move {
            match self.write_to_file().await {
                Ok(_) => {}
                Err(e) => {
                    error!("把缓存写入文件出错: {:?}", e)
                }
            }
        });
    }
}

impl CachePool {
    pub async fn from(config: &Config) -> Result<Self> {
        let limit_map: Arc<CacheMap> = Arc::new(create_map_by_config(config).await?);
        let strategy: ExpiredStrategy = if config.cache_get_strategy == 0 {
            Box::new(ExpiredCacheStrategy::from(limit_map.clone()))
        } else {
            Box::new(TimeoutCacheStrategy::from(limit_map.clone(),
                                                config.cache_ttl_timeout_ms as u128))
        };
        Ok(CachePool {
            strategy,
            file_name: config.cache_file.clone(),
            map: limit_map,
        })
    }
    pub fn get(&self, key: &Vec<u8>, get_value_fn: GetAnswerFunc) -> Result<DNSAnswer> {
        //从缓存map中取
        match self.map.get(key) {
            //缓存中有
            Some(r) => {
                self.strategy.handle(r, get_value_fn)
            }
            //缓存中没有
            None => {
                let answer = get_value_fn()?;
                if let Some(r) = (&answer).to_cache() {
                    self.map.insert(key.clone(), r);
                }
                Ok(answer)
            }
        }
    }

    fn to_file_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.map.iter().for_each(|e| {
            let bytes = e.value().to_bytes();
            vec.push(bytes.len() as u8);
            vec.extend(bytes);
        });
        vec.push(0);
        vec
    }

    pub async fn write_to_file(&self) -> Result<()> {
        if self.map.is_empty() {
            info!("没有缓存需要写入文件");
            return Ok(());
        }
        let mut file = File::create(&self.file_name).await?;
        file.write_all(self.to_file_bytes().as_slice()).await?;
        info!("缓存全部写入了文件! 文件名称是{}", self.file_name);
        Ok(())
    }
}

async fn create_map_by_config(config: &Config) -> Result<CacheMap> {
    Ok(match File::open(&config.cache_file).await {
        Ok(mut file) => {
            let mut file_vec = Vec::new();
            file.read_to_end(&mut file_vec).await?;
            if file_vec.is_empty() {
                LimitedMap::from(config.cache_num)
            } else {
                create_map_by_vec_u8(config, file_vec)
            }
        }
        Err(_e) => {
            LimitedMap::from(config.cache_num)
        }
    })
}

fn create_map_by_vec_u8(config: &Config, file_vec: Vec<u8>) -> CacheMap {
    let map = LimitedMap::from(config.cache_num);
    let mut cursor = Cursor::form(file_vec.into());
    let mut len = cursor.take() as usize;
    while len > 0 {
        let flag = cursor.peek();
        let record = match flag {
            IP_RECORD => {
                CacheRecord::from(IpCacheRecord::from(cursor.take_slice(len)))
            }
            SOA_RECORD => {
                CacheRecord::from(SoaCacheRecord::from(cursor.take_slice(len)))
            }
            _ => {
                panic!("Unsupported cache record!");
            }
        };
        if !record.is_expired(get_now()) {
            map.insert(record.get_key().clone(), record);
        }
        len = cursor.take() as usize;
    }
    map
}

#[cfg(test)]
mod tests {}