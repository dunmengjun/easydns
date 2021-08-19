use crate::protocol_new::question::Question;
use crate::cursor::Cursor;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct BasicData {
    question: Question,
    ttl: u32,
    data_len: u16,
}

impl From<&Cursor<u8>> for BasicData {
    fn from(cursor: &Cursor<u8>) -> Self {
        let question = Question::from(cursor);
        let ttl = u32::from_be_bytes(cursor.take_bytes());
        let data_len = u16::from_be_bytes(cursor.take_bytes());
        BasicData {
            question,
            ttl,
            data_len,
        }
    }
}

impl From<&BasicData> for Vec<u8> {
    fn from(data: &BasicData) -> Self {
        let question = &data.question;
        let mut vec: Vec<u8> = question.into();
        vec.extend(&data.ttl.to_be_bytes());
        vec.extend(&data.data_len.to_be_bytes());
        vec
    }
}

impl BasicData {
    pub fn get_name(&self) -> &String {
        &self.question.name
    }

    pub fn get_ttl(&self) -> u32 {
        self.ttl
    }

    pub fn get_type(&self) -> u16 {
        self.question._type
    }

    pub fn set_data_len(&mut self, len: u16) {
        self.data_len = len;
    }

    fn new() -> Self {
        let mut question = Question::new();
        question._type = 0;
        question.class = 1;
        BasicData {
            question,
            ttl: 0,
            data_len: 0,
        }
    }
}

pub struct Builder {
    data: Option<BasicData>,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            data: Some(BasicData::new())
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.data.as_mut().map(|e| {
            e.question.name = name;
            e
        });
        self
    }

    pub fn ttl(mut self, ttl: u32) -> Self {
        self.data.as_mut().map(|e| {
            e.ttl = ttl;
            e
        });
        self
    }

    pub fn _type(mut self, _type: u16) -> Self {
        self.data.as_mut().map(|e| {
            e.question._type = _type;
            e
        });
        self
    }

    pub fn data_len(mut self, data_len: u16) -> Self {
        self.data.as_mut().map(|e| {
            e.data_len = data_len;
            e
        });
        self
    }

    pub fn build(mut self) -> BasicData {
        self.data.take().unwrap()
    }
}