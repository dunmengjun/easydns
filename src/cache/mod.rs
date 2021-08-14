mod limit_map;
mod record;
mod expired_strategy;
mod timeout_strategy;

use crate::protocol::DNSAnswer;
use crate::config::Config;
use crate::system::{Result, get_now};
use std::sync::Arc;
use limit_map::{LimitedMap};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub use record::DNSCacheRecord;
use crate::cache::expired_strategy::ExpiredCacheStrategy;
use crate::cache::timeout_strategy::TimeoutCacheStrategy;

const F_DELIMITER: u8 = '|' as u8;
const F_SPACE: u8 = ' ' as u8;

pub trait CacheStrategy: Send + Sync {
    fn handle(&self, key: Vec<u8>, record: DNSCacheRecord,
              get_value_fn: Box<dyn FnOnce() -> Result<DNSAnswer> + Send + 'static>) -> Result<DNSAnswer>;
}

pub struct CachePool {
    disabled: bool,
    strategy: Box<dyn CacheStrategy>,
    file_name: String,
    map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>,
}

impl CachePool {
    pub async fn from(config: &Config) -> Result<Self> {
        let limit_map = Arc::new(if config.cache_on {
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
                self.strategy.handle(key.clone(), r, Box::new(get_value_fn))
            }
            //缓存中没有
            None => {
                let answer = get_value_fn()?;
                self.map.insert(key.clone(), answer.clone().into());
                Ok(answer)
            }
        }
    }

    fn to_file_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.map.iter().for_each(|e| {
            vec.extend(e.value().to_file_bytes());
        });
        vec.remove(vec.len() - 1);
        vec
    }

    pub async fn exit_process_action(&self) -> Result<()> {
        if self.disabled {
            info!("缓存已禁用");
            return Ok(());
        }
        if self.map.is_empty() {
            info!("没有缓存需要写入文件");
            return Ok(());
        }
        let mut file = File::create(&self.file_name).await?;
        file.write_all(self.to_file_bytes().as_slice()).await?;
        info!("缓存全部写入了文件! 文件名称是cache");
        Ok(())
    }
}

async fn create_map_by_config(config: &Config) -> Result<LimitedMap<Vec<u8>, DNSCacheRecord>> {
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

fn create_map_by_vec_u8(config: &Config, file_vec: Vec<u8>) -> LimitedMap<Vec<u8>, DNSCacheRecord> {
    let map = LimitedMap::from(config.cache_num);
    let split = file_vec.as_slice().split(|e| F_SPACE == *e);
    for r_bytes in split {
        let record = DNSCacheRecord::from_bytes(r_bytes);
        if !record.is_expired(get_now()) {
            map.insert(record.domain.clone(), record);
        }
    }
    map
}

#[cfg(test)]
mod tests {}