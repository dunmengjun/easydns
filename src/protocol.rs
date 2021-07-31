use std::fmt::{Debug};
use crate::buffer::PacketBuffer;

const C_FACTOR: u8 = 192u8;
const DC_FACTOR: u16 = 16383u16;

fn parse_name(buffer: &mut PacketBuffer, name_vec: &mut Vec<u8>) {
    if buffer.peek() & C_FACTOR == C_FACTOR {
        let c_index = u16::from_be_bytes([buffer.take(), buffer.take()]);
        buffer.tmp_at((c_index & DC_FACTOR) as usize, |buf| {
            parse_name(buf, name_vec);
        })
    } else {
        let seg_len = buffer.peek();
        if seg_len > 0 {
            let segment = buffer.take_slice(seg_len as usize + 1);
            name_vec.extend(segment);
            parse_name(buffer, name_vec)
        } else {
            name_vec.push(buffer.take());
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
    fn from(buffer: &mut PacketBuffer) -> Self {
        Header {
            id: u16::from_be_bytes([buffer.take(), buffer.take()]),
            flags: u16::from_be_bytes([buffer.take(), buffer.take()]),
            question_count: u16::from_be_bytes([buffer.take(), buffer.take()]),
            answer_count: u16::from_be_bytes([buffer.take(), buffer.take()]),
            authority_count: u16::from_be_bytes([buffer.take(), buffer.take()]),
            additional_count: u16::from_be_bytes([buffer.take(), buffer.take()]),
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

    fn from(buffer: &mut PacketBuffer) -> Self {
        let name = unzip_domain(buffer);
        let _type = u16::from_be_bytes([buffer.take(), buffer.take()]);
        let class = u16::from_be_bytes([buffer.take(), buffer.take()]);
        Question {
            name,
            _type,
            class,
        }
    }
}

#[derive(Debug)]
pub struct DNSQuery {
    header: Header,
    questions: Vec<Question>,
}

impl DNSQuery {
    pub fn from(mut cursor: PacketBuffer) -> Self {
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
    pub fn to_u8_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(self.header.to_u8_vec());
        self.questions.iter().for_each(|q| {
            bytes.extend(q.to_u8_vec())
        });
        bytes
    }

    pub fn get_id(&self) -> &u16 {
        &self.header.id
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
    fn from(buffer: &mut PacketBuffer) -> Self {
        let question = Question::from(buffer);
        let ttl = u32::from_be_bytes([buffer.take(),
            buffer.take(), buffer.take(), buffer.take()]);
        let data_len = u16::from_be_bytes([buffer.take(), buffer.take()]);
        let data = if question._type == 5 { //说明是cname类型
            unzip_domain(buffer)
        } else {
            Vec::<u8>::from(buffer.take_slice(data_len as usize))
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
pub struct DNSAnswer {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<ResourceRecord>,
}

impl DNSAnswer {
    pub fn from(mut buffer: PacketBuffer) -> Self {
        let header = Header::from(&mut buffer);
        let mut questions = Vec::new();
        (0..header.question_count as usize).into_iter().for_each(|_| {
            questions.push(Question::from(&mut buffer));
        });
        let mut resources = Vec::new();
        (0..header.answer_count as usize).into_iter().for_each(|_| {
            resources.push(ResourceRecord::from(&mut buffer));
        });
        DNSAnswer {
            header,
            questions,
            answers: resources,
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
        vec
    }

    pub fn get_id(&self) -> &u16 {
        &self.header.id
    }
}