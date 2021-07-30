mod cursor;

use std::net::UdpSocket;
use std::fmt::{Debug};
use std::sync::atomic::{AtomicU16, Ordering};
use crate::cursor::Cursor;

fn get_id() -> u16 {
    static mut ID_GEN: AtomicU16 = AtomicU16::new(0);
    unsafe {
        ID_GEN.fetch_add(1, Ordering::SeqCst)
    }
}

fn to_u8_to_push(vec: &mut Vec<u8>, data: u16) {
    let tmp = data.to_be_bytes();
    vec.push(tmp[0]);
    vec.push(tmp[1]);
}

fn parse_name(cursor: &mut Cursor, name_vec: &mut Vec<u8>) {
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

fn unzip_domain(cursor: &mut Cursor) -> Vec<u8> {
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
    fn new() -> Self {
        Header {
            id: get_id(),
            flags: 256, //00000001 00000000
            question_count: 1, //00000000 00000001
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        }
    }
    fn as_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(12);
        to_u8_to_push(&mut result, self.id);
        to_u8_to_push(&mut result, self.flags);
        to_u8_to_push(&mut result, self.question_count);
        to_u8_to_push(&mut result, self.answer_count);
        to_u8_to_push(&mut result, self.authority_count);
        to_u8_to_push(&mut result, self.additional_count);
        result
    }
    fn from(cursor: &mut Cursor) -> Self {
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
    fn new(domain: &str) -> Self {
        let mut question = Question {
            name: Vec::with_capacity(domain.len()),
            _type: 1,
            class: 1,
        };
        question.set_name(domain);
        question
    }
    fn as_bytes(&mut self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend(&self.name);
        to_u8_to_push(&mut result, self._type);
        to_u8_to_push(&mut result, self.class);
        result
    }
    fn set_name(&mut self, domain: &str) {
        for segment in domain.split('.') {
            self.name.push(segment.len() as u8);
            self.name.extend(segment.as_bytes());
        }
        self.name.push(0);
    }

    fn from(cursor: &mut Cursor) -> Self {
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

struct DNSQuery {
    header: Header,
    questions: Vec<Question>,
}

impl DNSQuery {
    fn new(domain: &str) -> Self {
        let question = Question::new(domain);
        let header = Header::new();
        Self {
            header,
            questions: vec![question],
        }
    }
    fn as_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(self.header.as_bytes());
        self.questions.iter_mut().for_each(|q| {
            bytes.extend(q.as_bytes())
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
    fn from(cursor: &mut Cursor) -> Self {
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
}

#[derive(Debug)]
struct DNSAnswer {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<ResourceRecord>,
}

impl DNSAnswer {
    fn from(cursor: &mut Cursor) -> Self {
        let header = Header::from(cursor);
        let mut questions = Vec::new();
        (0..header.question_count as usize).into_iter().for_each(|_| {
            questions.push(Question::from(cursor));
        });
        let mut resources = Vec::new();
        (0..header.answer_count as usize).into_iter().for_each(|_| {
            resources.push(ResourceRecord::from(cursor));
        });
        DNSAnswer {
            header,
            questions,
            answers: resources,
        }
    }
}

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:22222").unwrap();
    let mut query = DNSQuery::new("www.baidu.com");
    let query_vec = query.as_bytes();
    println!("send {:?}", &query_vec);
    match socket.send_to(query_vec.as_slice(), "114.114.114.114:53") {
        Ok(size) => println!("send ok {}", size),
        Err(e) => println!("send error {:?}", e)
    }
    let mut buf = [0u8; 1500];
    match socket.recv_from(&mut buf) {
        Ok(result) => {
            println!("size: {}, addr: {}", result.0, result.1);
            let answer = DNSAnswer::from(
                &mut Cursor::from(Vec::from(&buf[0..result.0]))
            );
            println!("answer {:?}", answer);
        }
        Err(e) => println!("res error {:?}", e)
    }
}
