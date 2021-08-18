use crate::protocol_new::header::Header;
use crate::protocol_new::question::Question;
use crate::cursor::Cursor;
use std::rc::Rc;

pub struct BasicData {
    header: Header,
    question: Question,
}

impl From<&Cursor<u8>> for BasicData {
    fn from(cursor: &Cursor<u8>) -> Self {
        let mut header = Header::from(cursor);
        if header.answer_count == 0 && header.authority_count == 0 {
            header.flags = 0x8182;
        }
        if header.question_count > 1 {
            panic!("不支持一个请求里有多个域名查询")
        }
        let question = Question::from(cursor);
        BasicData {
            header,
            question,
        }
    }
}

impl BasicData {
    pub fn set_id(&mut self, id: u16) {
        self.header.id = id;
    }
    pub fn set_answer_count(&mut self, count: u16) {
        self.header.answer_count = count
    }
    pub fn get_id(&self) -> u16 {
        self.header.id
    }

    pub fn get_flags(&self) -> u16 {
        self.header.flags
    }

    pub fn get_name(&self) -> &String {
        &self.question.name
    }

    fn new() -> Self {
        let mut header = Header::new();
        header.question_count = 1;
        let mut question = Question::new();
        question._type = 1;
        question.class = 1;
        BasicData {
            header,
            question,
        }
    }

    pub fn get_answer_count(&self) -> u16 {
        self.header.answer_count
    }

    pub fn get_authority_count(&self) -> u16 {
        self.header.authority_count
    }
}

impl From<&BasicData> for Vec<u8> {
    fn from(data: &BasicData) -> Self {
        let header = &data.header;
        let mut header_vec: Vec<u8> = header.into();
        let question = &data.question;
        let question_vec: Vec<u8> = question.into();
        header_vec.extend(question_vec);
        header_vec
    }
}

pub struct BasicDataBuilder {
    data: Option<BasicData>,
}

impl BasicDataBuilder {
    pub fn new() -> Self {
        BasicDataBuilder {
            data: Some(BasicData::new())
        }
    }

    pub fn id(mut self, id: u16) -> Self {
        self.data.as_mut().map(|e| {
            e.header.id = id;
            e
        });
        self
    }

    pub fn flags(mut self, flags: u16) -> Self {
        self.data.as_mut().map(|e| {
            e.header.flags = flags;
            e
        });
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.data.as_mut().map(|e| {
            e.question.name = name;
            e
        });
        self
    }

    pub fn authority(mut self, count: u16) -> Self {
        self.data.as_mut().map(|e| {
            e.header.authority_count = count;
            e
        });
        self
    }

    pub fn build(mut self) -> BasicData {
        self.data.take().unwrap()
    }
}