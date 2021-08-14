use crate::cache::limit_map::GetOrdKey;
use crate::cache::{F_DELIMITER, F_SPACE};
use crate::system::{get_now};
use crate::protocol::DNSAnswer;

#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub struct DNSCacheRecord {
    pub domain: Vec<u8>,
    pub address: Vec<u8>,
    pub start_time: u128,
    pub ttl_ms: u128,
}

impl DNSCacheRecord {
    pub fn is_expired(&self, now: u128) -> bool {
        let duration = now - self.start_time;
        self.ttl_ms < duration
    }

    pub fn get_remain_time(&self) -> u128 {
        let duration = get_now() - self.start_time;
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
        vec.extend(&(self.get_remain_time() as u32).to_be_bytes());
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
        self.get_remain_time()
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
            start_time: get_now(),
            ttl_ms: ttl,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::DNSCacheRecord;
    use crate::system::TIME;
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
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(999);
        });

        let result = record.get_remain_time();

        assert_eq!(1, result)
    }

    #[test]
    fn should_return_0_when_get_remain_time_given_expired() {
        let record = get_test_record();
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(1001);
        });

        let result = record.get_remain_time();

        assert_eq!(0, result)
    }

    #[test]
    fn should_return_valid_record_when_create_from_bytes_given_valid_bytes() {
        let vec = get_test_bytes();
        let valid_bytes = vec.as_slice();

        let result = DNSCacheRecord::from_bytes(valid_bytes);

        let expected = get_valid_record();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_bytes_when_to_from_bytes_given_valid_bytes() {
        let record = get_valid_record();

        let result = record.to_file_bytes();

        let expected = get_test_bytes();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_remain_time_when_get_order_key_given_test_record() {
        let record = get_test_record();
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

        let result = DNSCacheRecord::from(answer);

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

    fn get_valid_record() -> DNSCacheRecord {
        DNSCacheRecord {
            domain: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            address: vec![1, 1, 1, 1],
            start_time: 0,
            ttl_ms: 1000,
        }
    }

    fn get_test_record() -> DNSCacheRecord {
        DNSCacheRecord {
            domain: vec![],
            address: vec![],
            start_time: 0,
            ttl_ms: 1000,
        }
    }
}