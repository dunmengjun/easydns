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

const F_DELIMITER: u8 = '|' as u8;
const F_SPACE: u8 = ' ' as u8;

pub trait CacheStrategy: Send + Sync {
    fn handle(&self, key: Vec<u8>, record: CacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer>;
}

pub struct CachePool {
    disabled: bool,
    strategy: Box<dyn CacheStrategy>,
    file_name: String,
    map: Arc<LimitedMap<Vec<u8>, CacheRecord>>,
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
        let limit_map: Arc<LimitedMap<Vec<u8>, CacheRecord>> = Arc::new(if config.cache_on {
            create_map_by_config(config).await?
        } else {
            LimitedMap::from(0)
        });
        let strategy: Box<dyn CacheStrategy> = if config.cache_get_strategy == 0 {
            Box::new(ExpiredCacheStrategy::from(limit_map.clone()))
        } else {
            Box::new(TimeoutCacheStrategy::from(limit_map.clone(),
                                                config.cache_ttl_timeout_ms as u128))
        };
        Ok(CachePool {
            disabled: !config.cache_on,
            strategy,
            file_name: config.cache_file.clone(),
            map: limit_map,
        })
    }
    pub fn get(&self, key: &Vec<u8>, get_value_fn: impl FnOnce() -> Result<DNSAnswer> + Send + 'static)
               -> Result<DNSAnswer> {
        //如果缓存被禁用
        if self.disabled {
            return get_value_fn();
        }
        //从缓存map中取
        match self.map.get(key) {
            //缓存中有
            Some(r) => {
                self.strategy.handle(key.clone(), r.boxed_clone(), Box::new(get_value_fn))
            }
            //缓存中没有
            None => {
                let answer = get_value_fn()?;
                self.map.insert(key.clone(), answer.clone().into());
                Ok(answer)
            }
        }
    }

    pub async fn write_to_file(&self) -> Result<()> {
        if self.disabled {
            info!("缓存已禁用");
            return Ok(());
        }
        if self.map.is_empty() {
            info!("没有缓存需要写入文件");
            return Ok(());
        }
        let mut file = File::create(&self.file_name).await?;
        let vec: Vec<u8> = self.into();
        file.write_all(vec.as_slice()).await?;
        info!("缓存全部写入了文件! 文件名称是{}", self.file_name);
        Ok(())
    }
}

impl From<&CachePool> for Vec<u8> {
    fn from(pool: &CachePool) -> Self {
        let mut vec = Vec::new();
        pool.map.iter().for_each(|e| {
            let bytes: Vec<u8> = e.value().boxed_clone().into();
            vec.extend(bytes);
        });
        vec.remove(vec.len() - 1);
        vec
    }
}

async fn create_map_by_config(config: &Config) -> Result<LimitedMap<Vec<u8>, CacheRecord>> {
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

fn create_map_by_vec_u8(config: &Config, file_vec: Vec<u8>) -> LimitedMap<Vec<u8>, CacheRecord> {
    let map = LimitedMap::from(config.cache_num);
    let split = file_vec.as_slice().split(|e| F_SPACE == *e);
    for r_bytes in split {
        let record: CacheRecord = match r_bytes[0] {
            IP_RECORD => {
                Box::new(IpCacheRecord::from(r_bytes))
            }
            SOA_RECORD => {
                Box::new(SoaCacheRecord::from(r_bytes))
            }
            _ => {
                panic!("xx");
            }
        };
        if !record.is_expired(get_now()) {
            map.insert(record.get_key().clone(), record);
        }
    }
    map
}

#[cfg(test)]
mod tests {}