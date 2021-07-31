mod buffer;

use std::net::{UdpSocket};
use std::fmt::{Debug};
use crate::buffer::PacketBuffer;
use std::collections::HashMap;

fn parse_name(cursor: &mut PacketBuffer, name_vec: &mut Vec<u8>) {
    if cursor.peek() & 192u8 == 192u8 {
        let c_index = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let current_index_saved = cursor.get_current_index();
        cursor.at((c_index & 16383u16) as usize);
        parse_name(cursor, name_vec);
        cursor.at(current_index_saved);
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

fn unzip_domain(cursor: &mut PacketBuffer) -> Vec<u8> {
    let mut domain_vec = Vec::new();
    parse_name(cursor, &mut domain_vec);
    domain_vec
}

#[derive(Debug)]
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
        let mut result = Vec::with_capacity(12);
        result.extend(&self.id.to_be_bytes());
        result.extend(&self.flags.to_be_bytes());
        result.extend(&self.question_count.to_be_bytes());
        result.extend(&self.answer_count.to_be_bytes());
        result.extend(&self.authority_count.to_be_bytes());
        result.extend(&self.additional_count.to_be_bytes());
        result
    }
    fn from(cursor: &mut PacketBuffer) -> Self {
        Header {
            id: u16::from_be_bytes([cursor.take(), cursor.take()]),
            flags: u16::from_be_bytes([cursor.take(), cursor.take()]),
            question_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            answer_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            authority_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
            additional_count: u16::from_be_bytes([cursor.take(), cursor.take()]),
        }
    }
}

#[derive(Debug)]
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

    fn from(cursor: &mut PacketBuffer) -> Self {
        let name = unzip_domain(cursor);
        let _type = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let class = u16::from_be_bytes([cursor.take(), cursor.take()]);
        Question {
            name,
            _type,
            class,
        }
    }
}

#[derive(Debug)]
struct DNSQuery {
    header: Header,
    questions: Vec<Question>,
}

impl DNSQuery {
    fn from(mut cursor: PacketBuffer) -> Self {
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
    fn to_u8_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(self.header.to_u8_vec());
        self.questions.iter().for_each(|q| {
            bytes.extend(q.to_u8_vec())
        });
        bytes
    }
}

#[derive(Debug)]
struct ResourceRecord {
    name: Vec<u8>,
    _type: u16,
    class: u16,
    ttl: u32,
    data_len: u16,
    data: Vec<u8>,
}

impl ResourceRecord {
    fn from(cursor: &mut PacketBuffer) -> Self {
        let question = Question::from(cursor);
        let ttl = u32::from_be_bytes([cursor.take(),
            cursor.take(), cursor.take(), cursor.take()]);
        let data_len = u16::from_be_bytes([cursor.take(), cursor.take()]);
        let data = if question._type == 5 { //说明是cname类型
            unzip_domain(cursor)
        } else {
            Vec::<u8>::from(cursor.take_slice(data_len as usize))
        };
        ResourceRecord {
            name: question.name,
            _type: question._type,
            class: question.class,
            ttl,
            data_len,
            data,
        }
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

#[derive(Debug)]
struct DNSAnswer {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<ResourceRecord>,
}

impl DNSAnswer {
    fn from(mut cursor: PacketBuffer) -> Self {
        let header = Header::from(&mut cursor);
        let mut questions = Vec::new();
        (0..header.question_count as usize).into_iter().for_each(|_| {
            questions.push(Question::from(&mut cursor));
        });
        let mut resources = Vec::new();
        (0..header.answer_count as usize).into_iter().for_each(|_| {
            resources.push(ResourceRecord::from(&mut cursor));
        });
        DNSAnswer {
            header,
            questions,
            answers: resources,
        }
    }
    fn to_u8_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(self.header.to_u8_vec());
        self.questions.iter().for_each(|q| {
            bytes.extend(q.to_u8_vec())
        });
        self.answers.iter().for_each(|a| {
            bytes.extend(a.to_v8_vec())
        });
        bytes
    }
}

//dig @127.0.0.1 -p 2053 www.baidu.com
fn main() {
    let socket = UdpSocket::bind(("0.0.0.0", 2053)).unwrap();
    let mut query_map = HashMap::new();
    loop {
        let mut buffer = PacketBuffer::new();
        let (size, src) = socket.recv_from(buffer.as_mut_slice()).unwrap();
        if src.ip().to_string() == "114.114.114.114" {
            let answer = DNSAnswer::from(buffer);
            println!("dns answer: {:?}", answer);
            match query_map.get(&answer.header.id) {
                Some(_addr) => {
                    socket.send_to(answer.to_u8_vec().as_slice(), _addr).unwrap();
                    query_map.remove(&answer.header.id);
                }
                None => break
            };
        } else {
            let query = DNSQuery::from(buffer);
            println!("dns query: {:?}", query);
            socket.send_to(query.to_u8_vec().as_slice(), ("114.114.114.114", 53)).unwrap();
            query_map.insert(query.header.id, src);
        }
    }
}
