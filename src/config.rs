use crate::system::Result;

pub struct Config {
    pub cache_on: bool,
    pub cache_file: &'static str,
    pub cache_num: usize,
    pub port: usize,
    pub servers: Vec<&'static str>,
    pub filters: Vec<&'static str>,
    pub log_level: &'static str,
}

impl Config {
    fn new() -> Self {
        Config {
            cache_on: false,
            cache_file: "cache",
            cache_num: 1000,
            port: 2053,
            servers: vec![
                "114.114.114.114:53",
                "8.8.8.8:53",
                "1.1.1.1:53",
            ],
            filters: vec![
                "smartdns_anti_ad.conf.txt",
                "https://raw.githubusercontent.com/dunmengjun/SmartDNS-GFWList/master/smartdns_anti_ad.conf",
            ],
            log_level: "DEBUG",
        }
    }
}

pub async fn init_from_toml() -> Result<Config> {
    Ok(Config::new())
}