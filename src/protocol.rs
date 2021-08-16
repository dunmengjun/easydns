use std::fmt::{Debug, Display, Formatter};
use std::net::{IpAddr, Ipv4Addr};
use crate::system::{next_id, get_now, QueryBuf, AnswerBuf};
use crate::cache::{IpCacheRecord, SoaCacheRecord, CacheItem, CacheRecord};
use crate::cursor::{Cursor};

const C_FACTOR: u8 = 192u8;
const DC_FACTOR: u16 = 16383u16;

const QUERY_ONLY_RECURSIVELY: u16 = 0x0100;
const QUERY_RECURSIVELY_AD: u16 = 0x0120;

fn parse_name(cursor: &mut Cursor<u8>, name_vec: &mut Vec<u8>) {
    if cursor.peek() & C_FACTOR == C_FACTOR {
        let c_index = u16::from_be_bytes([cursor.take(), cursor.take()]);
        cursor.tmp_at((c_index & DC_FACTOR) as usize, |buf| {
            parse_name(buf, name_vec);
        })
    } else {
        let seg_len = cursor.peek();
        if seg_len > 0 {
            let segment = cursor.take_slice(seg_len as usize + 1);
            name_vec.extend(segment);
            parse_name(cursor, name_vec)
        } else {
            name_vec.push(cursor.take());
        }
    };
}

fn unzip_domain(cursor: &mut Cursor<u8>) -> Vec<u8> {
    let mut domain_vec = Vec::new();
    parse_name(cursor, &mut domain_vec);
    domain_vec
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Header {
    id: u16,
    flags: u16,
    question_count: u16,
    answer_count: u16,
    authority_count: u16,
    additional_count: u16,
}

impl Header {
    fn to_u8_vec(&self) -> Vec<u8> {
        self.to_u8_with_id(self.id)
    }
    fn to_u8_with_id(&self, id: u16) -> Vec<u8> {
        let mut result = Vec::with_capacity(12);
        result.extend(&id.to_be_bytes());
        result.extend(&self.flags.to_be_bytes());
        result.extend(&self.question_count.to_be_bytes());
        result.extend(&self.answer_count.to_be_bytes());
        result.extend(&self.authority_count.to_be_bytes());
        result.extend(&self.additional_count.to_be_bytes());
        result
    }
    fn from(cursor: &mut Cursor<u8>) -> Self {
        let header = Header {
            id: u16::from_be_bytes([cursor.take(), cursor.take()]),
            flags: u16::from_be_bytes([cursor.take(), cursor.take()]),
            question_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            answer_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            authority_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            additional_count: 0,
        };
        cursor.move_to(2);
        header
    }

    fn is_legal(&self) -> bool {
        !(self.answer_count > 0)
    }

    pub fn is_supported(&self) -> bool {
        let flag_supported =
            self.flags == QUERY_ONLY_RECURSIVELY
                || self.flags == QUERY_RECURSIVELY_AD;
        self.is_legal()
            && flag_supported
            && self.question_count == 1
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Question {
    name: Vec<u8>,
    _type: u16,
    class: u16,
}

impl Question {
    fn to_u8_vec(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend(&self.name);
        result.extend(&self._type.to_be_bytes());
        result.extend(&self.class.to_be_bytes());
        result
    }

    fn from(cursor: &mut Cursor<u8>) -> Self {
        let name = unzip_domain(cursor);
        let _type = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let class = u16::from_be_bytes([cursor.take(), cursor.take()]);
        Question {
            name,
            _type,
            class,
        }
    }

    fn is_legal(&self) -> bool {
        true
    }

    pub fn is_supported(&self) -> bool {
        self.is_legal()
            && self._type == 1
            && self.class == 1
    }
}

#[derive(Debug, Clone)]
pub struct DNSQuery {
    header: Header,
    questions: Vec<Question>,
}

impl DNSQuery {
    pub fn from(buf: QueryBuf) -> Self {
        let mut cursor = Cursor::form(buf.into());
        let header = Header::from(&mut cursor);
        let mut questions = Vec::new();
        (0..header.question_count as usize).into_iter().for_each(|_| {
            questions.push(Question::from(&mut cursor));
        });
        DNSQuery {
            header,
            questions,
        }
    }

    pub fn from_domain(domain: &str) -> Self {
        let header = Header {
            id: next_id(),
            flags: 0x0100,
            question_count: 1,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        };
        let question = Question {
            name: wrap_dns_domain(domain),
            _type: 1,
            class: 1,
        };
        DNSQuery {
            header,
            questions: vec![question],
        }
    }

    pub fn to_u8_with_id(&self, id: u16) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(self.header.to_u8_with_id(id));
        self.questions.iter().for_each(|q| {
            bytes.extend(q.to_u8_vec())
        });
        bytes
    }

    pub fn get_id(&self) -> &u16 {
        &self.header.id
    }

    pub fn get_domain(&self) -> &Vec<u8> {
        &self.questions[0].name
    }

    pub fn get_readable_domain(&self) -> String {
        let mut c = self.questions[0].name[0] as usize;
        let mut index = 1usize;
        let mut vec = Vec::new();
        while c != 0 {
            for i in index..(index + c) {
                vec.push(self.questions[0].name[i]);
            }
            vec.push('.' as u8);
            index += c;
            c = self.questions[0].name[index] as usize;
            index += 1
        }
        vec.remove(vec.len() - 1);
        String::from_utf8(vec).unwrap()
    }

    pub fn is_supported(&self) -> bool {
        self.header.is_supported()
            && self.questions.len() == 1
            && self.questions[0].is_supported()
    }
}

fn wrap_dns_domain(domain: &str) -> Vec<u8> {
    let mut vec = Vec::new();
    let split = domain.split(".");
    for str in split {
        vec.push(str.len() as u8);
        vec.extend(str.bytes());
    }
    vec.push(0u8);
    vec
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct ResourceRecord {
    name: Vec<u8>,
    _type: u16,
    class: u16,
    ttl: u32,
    data_len: u16,
    data: Vec<u8>,
}

impl ResourceRecord {
    fn from(cursor: &mut Cursor<u8>) -> Self {
        let question = Question::from(cursor);
        let ttl = u32::from_be_bytes([cursor.take(),
            cursor.take(), cursor.take(), cursor.take()]);
        let data_len = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let data = if question._type == 5 {
            //说明是cname类型
            unzip_domain(cursor)
        } else if question._type == 6 {
            //soa类型
            let old_index = cursor.get_current_index();
            let mut vec = Vec::new();
            vec.extend(unzip_domain(cursor));
            vec.extend(unzip_domain(cursor));
            let current_index = cursor.get_current_index();
            let len = current_index - old_index;
            let remained_slice = cursor.take_slice(data_len as usize - len);
            vec.extend(remained_slice);
            vec
        } else {
            Vec::<u8>::from(cursor.take_slice(data_len as usize))
        };
        ResourceRecord {
            name: question.name,
            _type: question._type,
            class: question.class,
            ttl,
            data_len: data.len() as u16,
            data,
        }
    }

    fn is_a_record(&self) -> bool {
        self._type == 1
    }

    fn to_v8_vec(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();
        result.extend(&self.name);
        result.extend(&self._type.to_be_bytes());
        result.extend(&self.class.to_be_bytes());
        result.extend(&self.ttl.to_be_bytes());
        result.extend(&(self.data.len() as u16).to_be_bytes());
        result.extend(&self.data);
        result
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DNSAnswer {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<ResourceRecord>,
    authorities: Vec<ResourceRecord>,
}

impl From<AnswerBuf> for DNSAnswer {
    fn from(buf: AnswerBuf) -> Self {
        let mut cursor = Cursor::form(buf.into());
        let mut header = Header::from(&mut cursor);
        let mut questions = Vec::new();
        (0..header.question_count as usize).into_iter().for_each(|_| {
            questions.push(Question::from(&mut cursor));
        });
        if header.answer_count == 0 && header.authority_count == 0 {
            header.flags = 0x8182;
        }
        let mut answers = Vec::new();
        (0..header.answer_count as usize).into_iter().for_each(|_| {
            answers.push(ResourceRecord::from(&mut cursor));
        });
        let mut authorities = Vec::new();
        (0..header.authority_count as usize).into_iter().for_each(|_| {
            authorities.push(ResourceRecord::from(&mut cursor))
        });
        DNSAnswer {
            header,
            questions,
            answers,
            authorities,
        }
    }
}

impl From<&IpCacheRecord> for DNSAnswer {
    fn from(record: &IpCacheRecord) -> Self {
        let mut questions = Vec::new();
        let mut answers = Vec::new();
        questions.push(Question {
            name: record.get_key().clone(),
            _type: 1,
            class: 1,
        });
        answers.push(ResourceRecord {
            name: record.get_key().clone(),
            _type: 1,
            class: 1,
            ttl: record.get_remain_time(get_now()) as u32 / 1000,
            data_len: 4,
            data: record.get_address().clone(),
        });
        DNSAnswer {
            header: Header {
                id: 0,
                flags: 0x8180,
                question_count: 1,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            questions,
            answers,
            authorities: vec![],
        }
    }
}

impl From<&SoaCacheRecord> for DNSAnswer {
    fn from(record: &SoaCacheRecord) -> Self {
        let mut questions = Vec::new();
        let mut authorities = Vec::new();
        questions.push(Question {
            name: record.get_key().clone(),
            _type: 1,
            class: 1,
        });
        let data = record.get_data();
        authorities.push(ResourceRecord {
            name: record.get_key().clone(),
            _type: 6,
            class: 1,
            ttl: record.get_remain_time(get_now()) as u32 / 1000,
            data_len: data.len() as u16,
            data: data.clone(),
        });
        DNSAnswer {
            header: Header {
                id: 0,
                flags: 0x8180,
                question_count: 1,
                answer_count: 0,
                authority_count: 1,
                additional_count: 0,
            },
            questions,
            answers: vec![],
            authorities,
        }
    }
}

impl DNSAnswer {
    pub fn from_query(query: &DNSQuery) -> Self {
        DNSAnswer {
            header: Header {
                id: query.header.id,
                flags: 0x8180,
                question_count: query.header.question_count,
                answer_count: 0,
                authority_count: 0,
                additional_count: 0,
            },
            questions: query.questions.clone(),
            answers: vec![],
            authorities: vec![],
        }
    }

    pub fn from_query_with_failure(query: &DNSQuery) -> Self {
        let header = Header {
            id: query.get_id().clone(),
            flags: 0x8182,
            question_count: 1,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        };
        let question = Question {
            name: query.get_domain().clone(),
            _type: 1,
            class: 1,
        };
        DNSAnswer {
            header,
            questions: vec![question],
            answers: vec![],
            authorities: vec![],
        }
    }

    pub fn from_query_with_soa(query: &DNSQuery) -> Self {
        let header = Header {
            id: query.get_id().clone(),
            flags: 0x8180,
            question_count: 1,
            answer_count: 0,
            authority_count: 1,
            additional_count: 0,
        };
        let question = Question {
            name: query.get_domain().clone(),
            _type: 1,
            class: 1,
        };
        let record = ResourceRecord {
            name: query.get_domain().clone(),
            _type: 6,
            class: 1,
            ttl: 0,
            data_len: 64,
            data: vec![
                0x01, 0x61, 0x0c, 0x67, 0x74, 0x6c, 0x64, 0x2d, 0x73, 0x65, 0x72, 0x76, 0x65, 0x72, 0x73, 0x03, 0x6e, 0x65, 0x74, 0x00,
                0x05, 0x6e, 0x73, 0x74, 0x6c, 0x64, 0x0c, 0x76, 0x65, 0x72, 0x69, 0x73, 0x69, 0x67, 0x6e, 0x2d, 0x67, 0x72, 0x73, 0x03,
                0x63, 0x6f, 0x6d, 0x00,
                0x00, 0x00, 0x07, 0x08, 0x00, 0x00, 0x07, 0x08, 0x00, 0x00, 0x03, 0x84, 0x00, 0x09, 0x3a, 0x80, 0x00, 0x01, 0x51, 0x80,
            ],
        };
        DNSAnswer {
            header,
            questions: vec![question],
            answers: vec![],
            authorities: vec![record],
        }
    }
    pub fn to_u8_vec(&self) -> Vec<u8> {
        let mut vec = Vec::<u8>::new();
        vec.extend(self.header.to_u8_vec());
        self.questions.iter().for_each(|q| {
            vec.extend(q.to_u8_vec())
        });
        self.answers.iter().for_each(|a| {
            vec.extend(a.to_v8_vec())
        });
        self.authorities.iter().for_each(|a| {
            vec.extend(a.to_v8_vec())
        });
        vec
    }

    pub fn get_id(&self) -> &u16 {
        &self.header.id
    }

    pub fn set_id(&mut self, id: u16) {
        self.header.id = id;
    }

    pub fn get_ip_vec(&self) -> Vec<IpAddr> {
        self.answers.iter().filter(|r| {
            r.is_a_record()
        }).map(|r| {
            let vec = &r.data;
            IpAddr::V4(Ipv4Addr::new(vec[0], vec[1], vec[2], vec[3]))
        }).collect()
    }

    pub fn retain_ip(&mut self, ip: IpAddr) {
        let ip_vec = match ip {
            IpAddr::V4(ipv4) => {
                Vec::from(ipv4.octets())
            }
            IpAddr::V6(ipv6) => {
                Vec::from(ipv6.octets())
            }
        };
        self.answers.retain(|r| {
            r.data.eq(&ip_vec)
        });
        let domain_name = &self.questions[0].name;
        self.answers.iter_mut().for_each(|mut r| {
            r.name = domain_name.clone();
        });
        self.header.answer_count = 1;
    }

    pub fn combine(&mut self, mut answer: DNSAnswer) {
        while let Some(mut r) = answer.answers.pop() {
            if r.is_a_record() {
                let flag = self.answers.iter().find(|e| {
                    e.data != r.data
                }).is_none();
                if flag {
                    r.name = self.questions[0].name.clone();
                    self.answers.push(r);
                }
            }
        }
        self.header.answer_count = self.answers.len() as u16;
    }

    pub fn is_empty_answers(&self) -> bool {
        self.answers.is_empty()
    }

    pub fn is_server_failure(&self) -> bool {
        self.header.flags == 0x8182
    }

    pub fn is_no_such_name(&self) -> bool {
        self.header.flags == 0x8183
    }

    pub fn to_cache(&self) -> Option<CacheRecord> {
        if self.is_server_failure() {
            return None;
        }
        if self.is_no_such_name() {
            return None;
        }
        let t: CacheRecord = if !self.is_empty_answers() {
            Box::new(IpCacheRecord {
                domain: self.questions[0].name.clone(),
                address: self.answers[0].data.clone(),
                create_time: get_now(),
                ttl_ms: self.answers[0].ttl as u128 * 1000,
            })
        } else {
            Box::new(SoaCacheRecord {
                domain: self.questions[0].name.clone(),
                data: self.authorities[0].data.clone(),
                create_time: get_now(),
                ttl_ms: self.authorities[0].ttl as u128 * 1000,
            })
        };
        Some(t)
    }

    fn get_readable_domain(&self) -> String {
        let vec = self.questions[0].name.clone();
        let mut cursor = Cursor::<u8>::form(Box::new(vec));
        let mut flag = cursor.take();
        let mut buf = String::new();
        while flag > 0 {
            let str = cursor.take_slice(flag as usize);
            let result = String::from_utf8(Vec::from(str)).unwrap();
            buf.push_str(&result);
            buf.push('.');
            flag = cursor.take();
        }
        buf.remove(buf.len() - 1);
        buf
    }
}

impl Display for DNSAnswer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let domain = self.get_readable_domain();
        if self.is_server_failure() {
            write!(f, "(FAILURE, {})", domain)
        } else if self.is_no_such_name() {
            write!(f, "(NO_SUCH_NAME, {})", domain)
        } else if !self.is_empty_answers() {
            write!(f, "(IP, {}, {}, {:?})", domain, self.answers[0].ttl, self.answers[0].data)
        } else {
            write!(f, "(SOA, {}, {})", domain, self.authorities[0].ttl)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::protocol::{DNSAnswer, ResourceRecord, Header, Question};

    pub fn get_ip_answer() -> DNSAnswer {
        let question = Question {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 1,
            class: 1,
        };
        let record = ResourceRecord {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 1,
            class: 1,
            ttl: 1,
            data_len: 4,
            data: vec![1, 1, 1, 1],
        };
        DNSAnswer {
            header: Header {
                id: 0,
                flags: 0x8180,
                question_count: 1,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            questions: vec![question],
            answers: vec![record],
            authorities: vec![],
        }
    }

    pub fn get_soa_answer() -> DNSAnswer {
        let question = Question {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 1,
            class: 1,
        };
        let record = ResourceRecord {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 6,
            class: 1,
            ttl: 1,
            data_len: 4,
            data: vec![1, 1, 1, 1],
        };
        DNSAnswer {
            header: Header {
                id: 0,
                flags: 0x8180,
                question_count: 1,
                answer_count: 0,
                authority_count: 1,
                additional_count: 0,
            },
            questions: vec![question],
            answers: vec![],
            authorities: vec![record],
        }
    }

    pub fn get_ip_answer_with_ttl(ttl: u32) -> DNSAnswer {
        let question = Question {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 1,
            class: 1,
        };
        let record = ResourceRecord {
            name: vec![3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0],
            _type: 1,
            class: 1,
            ttl,
            data_len: 4,
            data: vec![1, 1, 1, 1],
        };
        DNSAnswer {
            header: Header {
                id: 0,
                flags: 0x8180,
                question_count: 1,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            questions: vec![question],
            answers: vec![record],
            authorities: vec![],
        }
    }
}
