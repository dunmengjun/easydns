use crate::cursor::Cursor;
use crate::protocol::{unzip_domain, wrap_name};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Question {
    pub name: String,
    pub _type: u16,
    pub class: u16,
}

impl From<&Question> for Vec<u8> {
    fn from(question: &Question) -> Self {
        let mut result = Vec::new();
        result.extend(wrap_name(&question.name));
        result.extend(&question._type.to_be_bytes());
        result.extend(&question.class.to_be_bytes());
        result
    }
}

impl From<&Cursor<u8>> for Question {
    fn from(cursor: &Cursor<u8>) -> Self {
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

impl Question {
    fn is_legal(&self) -> bool {
        true
    }

    pub fn is_supported(&self) -> bool {
        self.is_legal()
            && self._type == 1
            && self.class == 1
    }

    pub fn new() -> Self {
        Question {
            name: String::new(),
            _type: 0,
            class: 0,
        }
    }
}