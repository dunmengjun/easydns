use crate::cache::cache_record::{CacheItem, SOA_RECORD};
use crate::cache::{F_DELIMITER};
use crate::system::get_now;
use crate::protocol::DNSAnswer;

#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub struct SoaCacheRecord {
    pub domain: Vec<u8>,
    pub data: Vec<u8>,
    pub create_time: u128,
    pub ttl_ms: u128,
}

impl CacheItem for SoaCacheRecord {
    fn get_create_time(&self) -> u128 {
        self.create_time
    }

    fn get_ttl_ms(&self) -> u128 {
        self.ttl_ms
    }

    fn get_key(&self) -> &Vec<u8> {
        &self.domain
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.into()
    }

    fn to_answer(&self) -> DNSAnswer {
        DNSAnswer::from(self)
    }
}

impl SoaCacheRecord {
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl From<&SoaCacheRecord> for Vec<u8> {
    fn from(record: &SoaCacheRecord) -> Self {
        let mut vec = Vec::<u8>::new();
        //插入魔数
        vec.push(SOA_RECORD);
        vec.push(F_DELIMITER);
        vec.extend(&record.domain);
        vec.push(F_DELIMITER);
        vec.extend(&(record.get_remain_time(get_now()) as u32).to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&record.create_time.to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&record.data);
        vec
    }
}

impl From<&[u8]> for SoaCacheRecord {
    fn from(bytes: &[u8]) -> Self {
        let split: Vec<&[u8]> = bytes.split(|e| F_DELIMITER == *e).collect();
        let domain = Vec::<u8>::from(split[1]);
        let mut buf = [0u8; 4];
        for i in 0..4 {
            buf[i] = split[2][i]
        }
        let ttl_ms = u32::from_be_bytes(buf) as u128;
        let mut buf = [0u8; 16];
        for i in 0..16 {
            buf[i] = split[3][i];
        }
        let create_time = u128::from_be_bytes(buf);
        let data = Vec::<u8>::from(split[4]);
        SoaCacheRecord {
            domain,
            data,
            create_time,
            ttl_ms,
        }
    }
}