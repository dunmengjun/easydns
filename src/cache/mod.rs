mod limit_map;
mod record;

use crate::protocol::DNSAnswer;
use crate::config::Config;
use crate::system::{Result, TimeNow};
use std::sync::Arc;
use limit_map::{LimitedMap};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;

pub use record::DNSCacheRecord;

const F_DELIMITER: u8 = '|' as u8;
const F_SPACE: u8 = ' ' as u8;

pub struct CachePool {
    disabled: bool,
    strategy: usize,
    timeout: u128,
    file_name: String,
    map: Arc<LimitedMap<Vec<u8>, DNSCacheRecord>>,
}

impl CachePool {
    pub async fn from(config: &Config) -> Result<Self> {
        let limit_map = if config.cache_on {
            create_map_by_config(config).await?
        } else {
            LimitedMap::new()
        };
        Ok(CachePool {
            disabled: !config.cache_on,
            strategy: config.cache_get_strategy,
            timeout: config.cache_ttl_timeout_ms as u128,
            file_name: config.cache_file.clone(),
            map: Arc::new(limit_map),
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
                self.get_with_strategy(key, r, get_value_fn)
            }
            //缓存中没有
            None => {
                self.sync_get_and_insert(key.clone(), get_value_fn)
            }
        }
    }

    fn sync_get_and_insert(&self, key: Vec<u8>, get_value_fn: impl FnOnce() -> Result<DNSAnswer>) -> Result<DNSAnswer> {
        let answer = get_value_fn()?;
        self.map.insert(key, answer.clone().into());
        Ok(answer)
    }

    fn async_get_and_insert(&self, key: Vec<u8>, get_value_fn: impl FnOnce() -> Result<DNSAnswer> + Send + 'static) {
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
    }
    fn get_with_strategy(&self,
                         key: &Vec<u8>,
                         value: DNSCacheRecord,
                         get_value_fn: impl FnOnce() -> Result<DNSAnswer> + Send + 'static) -> Result<DNSAnswer> {
        if value.is_expired(TimeNow::new()) {
            if self.strategy == 0 {
                self.map.remove(key);
                self.sync_get_and_insert(key.clone(), get_value_fn)
            } else {
                //超时测试
                let timeout = TimeNow::new().add(Duration::from_millis(self.timeout as u64));
                if value.is_expired(timeout) {
                    self.async_get_and_insert(key.clone(), get_value_fn);
                }
                Ok(value.into())
            }
        } else {
            Ok(value.into())
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
        if !record.is_expired(TimeNow::new()) {
            map.insert(record.domain.clone(), record);
        }
    }
    map
}