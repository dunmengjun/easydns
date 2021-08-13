use crate::cache::limit_map::GetOrdKey;
use crate::cache::{F_DELIMITER, F_SPACE};
use crate::system::TimeNow;
use crate::protocol::DNSAnswer;

#[derive(Clone, PartialOrd, PartialEq)]
pub struct DNSCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
    pub start_time: u128,
    pub ttl_ms: u128,
}

impl DNSCacheRecord {
    pub fn is_expired(&self, time: TimeNow) -> bool {
        let duration = time.get() - self.start_time;
        self.ttl_ms < duration
    }

    pub fn get_remain_time(&self, now: TimeNow) -> u128 {
        let duration = now.get() - self.start_time;
        if self.ttl_ms > duration {
            self.ttl_ms - duration
        } else {
            0
        }
    }

    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.domain
    }

    pub fn to_file_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::<u8>::new();
        vec.extend(&self.domain);
        vec.push(F_DELIMITER);
        vec.extend(&self.address);
        vec.push(F_DELIMITER);
        vec.extend(&(self.get_remain_time(TimeNow::new()) as u32).to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&self.start_time.to_be_bytes());
        vec.push(F_SPACE);
        vec
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
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
        let start_time = u128::from_be_bytes(buf);
        DNSCacheRecord {
            domain,
            address,
            start_time,
            ttl_ms: ttl,
        }
    }
}

impl GetOrdKey for DNSCacheRecord {
    type Output = u128;
    fn get_order_key(&self) -> Self::Output {
        self.get_remain_time(TimeNow::new())
    }
}

impl From<DNSAnswer> for DNSCacheRecord {
    fn from(answer: DNSAnswer) -> Self {
        let domain = answer.get_domain().clone();
        let ttl = answer.get_ttl_secs() as u128 * 1000;
        let address = answer.get_address().clone();
        DNSCacheRecord {
            domain,
            address,
            start_time: TimeNow::new().get(),
            ttl_ms: ttl,
        }
    }
}