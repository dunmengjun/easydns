use crate::cache::cache_record::{CacheItem, SOA_RECORD};
use crate::system::get_now;
use crate::protocol::DNSAnswer;
use crate::cursor::Cursor;

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
        vec.push(SOA_RECORD);//插入魔数
        vec.push(record.domain.len() as u8);
        vec.extend(&record.domain);
        vec.push(4);
        vec.extend(&(record.get_remain_time(get_now()) as u32).to_be_bytes());
        vec.push(16);
        vec.extend(&record.create_time.to_be_bytes());
        vec.push(record.data.len() as u8);
        vec.extend(&record.data);
        vec
    }
}

impl From<&[u8]> for SoaCacheRecord {
    fn from(bytes: &[u8]) -> Self {
        let mut cursor = Cursor::form(Vec::from(bytes).into());
        cursor.take();//删掉魔数
        let mut len = cursor.take() as usize;
        let domain = Vec::from(cursor.take_slice(len));
        cursor.take();
        let ttl_ms = u32::from_be_bytes(cursor.take_bytes()) as u128;
        cursor.take();
        let create_time = u128::from_be_bytes(cursor.take_bytes());
        len = cursor.take() as usize;
        let data = Vec::from(cursor.take_slice(len));
        SoaCacheRecord {
            domain,
            data,
            create_time,
            ttl_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::{SoaCacheRecord, CacheRecord, CacheItem};
    use crate::system::TIME;
    use crate::protocol::tests::{get_ip_answer, get_soa_answer};

    #[test]
    fn should_return_valid_record_when_create_from_bytes_given_valid_bytes() {
        let vec = get_test_bytes();
        let valid_bytes = vec.as_slice();

        let result = SoaCacheRecord::from(valid_bytes);

        let expected = get_soa_record();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_bytes_when_to_from_bytes_given_valid_bytes() {
        let record = get_soa_record();

        let result = record.to_bytes();

        let expected = get_test_bytes();
        assert_eq!(expected, result)
    }

    #[test]
    fn should_return_valid_record_when_from_answer_given_valid_answer() {
        let answer = get_soa_answer();
        TIME.with(|t| {
            t.borrow_mut().set_timestamp(0);
        });

        let result = answer.to_cache().unwrap();

        let expected: CacheRecord = get_soa_record().into();
        assert!(expected.eq(&result))
    }

    fn get_test_bytes() -> Vec<u8> {
        let bytes: [u8; 44] = [35, 15, 3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 4, 0, 0, 3, 232, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 1, 1, 1, 1];
        let mut vec = Vec::with_capacity(44);
        for c in bytes.iter() {
            vec.push(c.clone())
        }
        vec
    }

    pub fn get_soa_record() -> SoaCacheRecord {
        SoaCacheRecord {
            domain: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            data: vec![1, 1, 1, 1],
            create_time: 0,
            ttl_ms: 1000,
        }
    }

    pub fn build_soa_record(f: fn(&mut SoaCacheRecord)) -> SoaCacheRecord {
        let mut record = get_soa_record();
        f(&mut record);
        record
    }
}