use crate::system::{get_now};
use crate::cache::cache_record::{CacheItem, IP_RECORD};
use crate::cursor::Cursor;
use crate::protocol::{DnsAnswer, Ipv4Answer};
use std::net::Ipv4Addr;

#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub struct IpCacheRecord {
    pub domain: String,
    pub address: Ipv4Addr,
    pub create_time: u128,
    pub ttl_ms: u128,
}

impl CacheItem for IpCacheRecord {
    fn get_create_time(&self) -> u128 {
        self.create_time
    }

    fn get_ttl_ms(&self) -> u128 {
        self.ttl_ms
    }

    fn get_key(&self) -> &String {
        &self.domain
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.into()
    }

    fn to_answer(&self) -> DnsAnswer {
        Ipv4Answer::from(self).into()
    }
}

impl IpCacheRecord {
    pub fn get_address(&self) -> &Ipv4Addr {
        &self.address
    }
}

impl From<&IpCacheRecord> for Vec<u8> {
    fn from(record: &IpCacheRecord) -> Self {
        let mut vec = Vec::<u8>::new();
        vec.push(IP_RECORD);//插入魔数
        vec.push(record.domain.len() as u8);
        vec.extend(record.domain.as_bytes());
        vec.push(4);
        vec.extend(&(record.get_remain_time(get_now()) as u32).to_be_bytes());
        vec.push(16);
        vec.extend(&record.create_time.to_be_bytes());
        vec.push(4);
        vec.extend(&record.address.octets());
        vec
    }
}

impl From<&[u8]> for IpCacheRecord {
    fn from(bytes: &[u8]) -> Self {
        let cursor = Cursor::form(Vec::from(bytes).into());
        cursor.take(); //删掉魔数
        let len = cursor.take() as usize;
        let domain = String::from_utf8(Vec::from(cursor.take_slice(len))).unwrap();
        cursor.take();
        let ttl_ms = u32::from_be_bytes(cursor.take_bytes()) as u128;
        cursor.take();
        let create_time = u128::from_be_bytes(cursor.take_bytes());
        cursor.take();
        let address = Ipv4Addr::from(cursor.take_bytes());
        IpCacheRecord {
            domain,
            address,
            create_time,
            ttl_ms,
        }
    }
}

impl From<&Ipv4Answer> for IpCacheRecord {
    fn from(answer: &Ipv4Answer) -> Self {
        IpCacheRecord {
            domain: answer.get_name().clone(),
            address: answer.get_address().clone(),
            create_time: get_now(),
            ttl_ms: answer.get_ttl() as u128 * 1000,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::cache::{IpCacheRecord, CacheItem, CacheRecord};
    use crate::system::{TIME, get_now};
    use crate::cache::limit_map::GetOrdKey;
    use crate::protocol::tests::get_ip_answer;
    use std::net::Ipv4Addr;

    #[test]
    fn should_return_valid_record_when_create_from_bytes_given_valid_bytes() {
        let vec = get_test_bytes();
        let valid_bytes = vec.as_slice();

        let result = IpCacheRecord::from(valid_bytes);

        let expected = get_ip_record();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_bytes_when_to_from_bytes_given_valid_bytes() {
        let record = get_ip_record();

        let result = record.to_bytes();

        let expected = get_test_bytes();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_valid_record_when_from_answer_given_valid_answer() {
        let answer = get_ip_answer();
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(0);
        });

        let result = answer.to_cache().unwrap();

        let expected: CacheRecord = get_ip_record().into();
        assert!(expected.eq(&result))
    }

    fn get_test_bytes() -> Vec<u8> {
        let bytes: [u8; 44] = [42, 15, 3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 4, 0, 0, 3, 232, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 1, 1, 1, 1];
        let mut vec = Vec::with_capacity(44);
        for c in bytes.iter() {
            vec.push(c.clone())
        }
        vec
    }

    pub fn get_ip_record() -> IpCacheRecord {
        IpCacheRecord {
            domain: "www.baidu.com".to_string(),
            address: Ipv4Addr::from([1, 1, 1, 1]),
            create_time: 0,
            ttl_ms: 1000,
        }
    }
}