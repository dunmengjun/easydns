use std::net::UdpSocket;
use std::sync::{Mutex};
use std::fmt::{Debug};

fn get_id() -> u16 {
    static mut ID_GEN: u16 = 0;
    unsafe {
        let mutex = Mutex::new(10);
        mutex.lock();
        ID_GEN = ID_GEN + 1;
        if ID_GEN > u16::MAX {
            ID_GEN = 0
        }
        ID_GEN
    }
}

fn to_u8_to_push(vec: &mut Vec<u8>, data: u16) {
    let mut tmp = data.to_be_bytes();
    vec.push(tmp[0]);
    vec.push(tmp[1]);
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
        let mut result = Vec::new();
        to_u8_to_push(&mut result, self.id);
        to_u8_to_push(&mut result, self.flags);
        to_u8_to_push(&mut result, self.question_count);
        to_u8_to_push(&mut result, self.answer_count);
        to_u8_to_push(&mut result, self.authority_count);
        to_u8_to_push(&mut result, self.additional_count);
        result
    }
    fn from(src: &[u8]) -> Self {
        Header {
            id: u16::from_be_bytes([src[0], src[1]]),
            flags: u16::from_be_bytes([src[2], src[3]]),
            question_count: u16::from_be_bytes([src[4], src[5]]),
            answer_count: u16::from_be_bytes([src[6], src[7]]),
            authority_count: u16::from_be_bytes([src[8], src[9]]),
            additional_count: u16::from_be_bytes([src[10], src[11]]),
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
            name: vec![],
            _type: 1,
            class: 1,
        };
        question.set_name(domain);
        question
    }
    fn as_bytes(&mut self) -> Vec<u8> {
        let mut result = Vec::new();
        let x = self._type.to_be_bytes();
        let y = self.class.to_be_bytes();
        result.extend(&self.name);
        result.push(x[0]);
        result.push(x[1]);
        result.push(y[0]);
        result.push(y[1]);
        result
    }
    fn set_name(&mut self, domain: &str) {
        let split = domain.split('.');
        for segment in split {
            self.name.push(segment.len() as u8);
            let seg_u8 = segment.as_bytes();
            for c in seg_u8 {
                self.name.push(c.clone());
            }
        }
        self.name.push(0);
    }

    fn from(buf: &[u8]) -> (Self, usize) {
        let mut current_index = 0;
        let mut seg_len = buf[current_index] as usize;
        let mut name = Vec::new();
        while seg_len > 0 {
            let segment = &buf[current_index..current_index + seg_len + 1];
            name.extend(segment);
            current_index = current_index + seg_len + 1;
            seg_len = buf[current_index] as usize;
        }
        name.push(buf[current_index]);
        current_index += 1;
        let _type = u16::from_be_bytes([buf[current_index], buf[current_index + 1]]);
        let class = u16::from_be_bytes([buf[current_index + 2], buf[current_index + 3]]);
        (Question {
            name,
            _type,
            class,
        }, current_index + 4)
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
    fn from(buf: &[u8]) -> (Self, usize) {
        let result = Question::from(&buf[0..buf.len()]);
        let mut index = result.1;
        let ttl = u32::from_be_bytes([buf[index], buf[index + 1], buf[index + 2], buf[index + 3]]);
        index += 4;
        let data_len = u16::from_be_bytes([buf[index], buf[index + 1]]);
        index += 2;
        let data = Vec::<u8>::from(&buf[index..index + data_len as usize]);
        (ResourceRecord {
            name: result.0.name,
            _type: result.0._type,
            class: result.0.class,
            ttl,
            data_len,
            data,
        }, index + data_len as usize)
    }
}

#[derive(Debug)]
struct DNSAnswer {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<ResourceRecord>,
}

impl DNSAnswer {
    fn from(buf: &[u8]) -> Self {
        let header = Header::from(&buf[0..12]);
        let mut current_index = 12;
        let mut questions = Vec::new();
        for _ in 0..header.question_count as usize {
            let result = Question::from(&buf[current_index..buf.len()]);
            questions.push(result.0);
            current_index += result.1
        }
        let mut resources = Vec::new();
        for _ in 0..header.answer_count as usize {
            let result = ResourceRecord::from(&buf[current_index..buf.len()]);
            resources.push(result.0);
            current_index += result.1;
        }
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
    let vec = query.as_bytes();
    println!("send {:?}", &vec);
    match socket.send_to(vec.as_slice(), "192.168.123.1:53") {
        Ok(size) => println!("send ok {}", size),
        Err(e) => println!("send error {:?}", e)
    }
    let mut buf = [0u8; 1500];
    match socket.recv_from(&mut buf) {
        Ok(result) => {
            println!("size: {}, addr: {}", result.0, result.1);
            let answer = DNSAnswer::from(&buf);
            println!("answer {:?}", answer);
        }
        Err(e) => println!("res error {:?}", e)
    }
}
