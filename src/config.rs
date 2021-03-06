use crate::system::Result;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use toml::Value;

pub struct Config {
    pub cache_on: bool,
    pub cache_file: String,
    pub cache_num: usize,
    pub port: u16,
    pub servers: Vec<String>,
    pub filters: Vec<String>,
    pub log_level: String,
    pub ip_choose_strategy: usize,
    pub cache_get_strategy: usize,
    pub cache_ttl_timeout_ms: usize,
    pub server_choose_strategy: usize,
    pub server_choose_duration_h: usize,
}

impl Config {
    fn from(value: Value) -> Self {
        let cache_file = value["cache-file"].as_str().map(|e| String::from(e))
            .unwrap_or("cache".into());
        let cache_on = value["cache"].as_bool().unwrap_or(true);
        let cache_num = value["cache-num"].as_integer().unwrap_or(1000) as usize;
        let port = value["port"].as_integer().unwrap_or(2053) as u16;
        let servers = value["servers"].as_array().map(|e| {
            e.iter().map(|e| String::from(e.as_str().unwrap())).collect()
        }).unwrap_or(vec![]);
        let filters = value["filters"].as_array().map(|e| {
            e.iter().map(|e| String::from(e.as_str().unwrap())).collect()
        }).unwrap_or(vec![]);
        let log_level = value["log-level"].as_str().map(|e| String::from(e))
            .unwrap_or("error".into());
        let ip_choose_strategy = value["ip-choose-strategy"].as_integer()
            .unwrap_or(0) as usize;
        let cache_get_strategy = value["cache-get-strategy"].as_integer()
            .unwrap_or(0) as usize;
        let cache_ttl_timeout_ms = value["cache-ttl-timeout-ms"].as_integer()
            .unwrap_or(0) as usize;
        let server_choose_strategy = value["server-choose-strategy"].as_integer()
            .unwrap_or(0) as usize;
        let server_choose_duration_h = value["server-choose-duration-h"].as_integer()
            .unwrap_or(12) as usize;
        Config {
            cache_on,
            cache_file,
            cache_num,
            port,
            servers,
            filters,
            log_level,
            ip_choose_strategy,
            cache_get_strategy,
            cache_ttl_timeout_ms,
            server_choose_strategy,
            server_choose_duration_h,
        }
    }
}

pub async fn init_from_toml() -> Result<Config> {
    let mut file = File::open("easydns.toml").await?;
    let buf = &mut String::new();
    file.read_to_string(buf).await?;
    Ok(Config::from(buf.parse::<Value>()?))
}
