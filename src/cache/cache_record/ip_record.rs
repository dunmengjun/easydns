use crate::cache::{F_DELIMITER, F_SPACE};
use crate::system::{get_now};
use crate::protocol::DNSAnswer;
use crate::cache::cache_record::{CacheItem, IP_RECORD};

#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub struct IpCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
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

impl IpCacheRecord {
    pub fn get_address(&self) -> &Vec<u8> {
        &self.address
    }
}

impl From<&IpCacheRecord> for Vec<u8> {
    fn from(record: &IpCacheRecord) -> Self {
        let mut vec = Vec::<u8>::new();
        //插入魔数
        vec.push(IP_RECORD);
        vec.push(F_DELIMITER);
        vec.extend(&record.domain);
        vec.push(F_DELIMITER);
        vec.extend(&(record.get_remain_time(get_now()) as u32).to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&record.create_time.to_be_bytes());
        vec.push(F_DELIMITER);
        vec.extend(&record.address);
        vec.push(F_SPACE);
        vec
    }
}

impl From<&[u8]> for IpCacheRecord {
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
        let address = Vec::<u8>::from(split[4]);
        IpCacheRecord {
            domain,
            address,
            create_time,
            ttl_ms,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::cache::{DNSCacheRecord, IpCacheRecord, CacheItem, CacheRecord};
    use crate::system::{TIME, get_now};
    use crate::cache::limit_map::GetOrdKey;
    use crate::protocol::tests::get_valid_answer;

    #[test]
    fn should_return_true_when_check_expired_given_expired() {
        let record = get_test_record();

        let result = record.is_expired(1001);

        assert!(result)
    }

    #[test]
    fn should_return_false_when_check_expired_given_not_expired() {
        let record = get_test_record();

        let result = record.is_expired(999);

        assert!(!result)
    }

    #[test]
    fn should_return_remain_time_when_get_remain_time_given_not_expired() {
        let record = get_test_record();

        let result = record.get_remain_time(999);

        assert_eq!(1, result)
    }

    #[test]
    fn should_return_0_when_get_remain_time_given_expired() {
        let record = get_test_record();

        let result = record.get_remain_time(1001);

        assert_eq!(0, result)
    }

    #[test]
    fn should_return_valid_record_when_create_from_bytes_given_valid_bytes() {
        let vec = get_test_bytes();
        let valid_bytes = vec.as_slice();

        let result = IpCacheRecord::from(valid_bytes);

        let expected = get_valid_record();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_bytes_when_to_from_bytes_given_valid_bytes() {
        let record = get_valid_record();

        let result = record.to_bytes();

        let expected = get_test_bytes();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_remain_time_when_get_order_key_given_test_record() {
        let record: CacheRecord = Box::new(get_test_record());
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(999);
        });

        let result: u128 = record.get_order_key();

        assert_eq!(1, result)
    }

    #[test]
    fn should_return_valid_record_when_from_answer_given_valid_answer() {
        let answer = get_valid_answer();
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(0);
        });

        let result = IpCacheRecord::from(answer);

        let expected = get_valid_record();
        assert_eq!(expected, result)
    }

    fn get_test_bytes() -> Vec<u8> {
        let bytes: [u8; 43] = [3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 124, 1, 1, 1, 1, 124, 0, 0, 3, 232, 124, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32];
        let mut vec = Vec::with_capacity(43);
        for c in bytes.iter() {
            vec.push(c.clone())
        }
        vec
    }

    pub fn get_valid_record() -> IpCacheRecord {
        IpCacheRecord {
            domain: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            address: vec![1, 1, 1, 1],
            create_time: 0,
            ttl_ms: 1000,
        }
    }

    pub fn build_valid_record(f: fn(&mut IpCacheRecord)) -> IpCacheRecord {
        let mut record = get_valid_record();
        f(&mut record);
        record
    }

    fn get_test_record() -> IpCacheRecord {
        IpCacheRecord {
            domain: vec![],
            address: vec![],
            create_time: 0,
            ttl_ms: 1000,
        }
    }
}