use crate::cursor::Cursor;
use crate::protocol_new::unzip_domain;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Question {
    pub name: String,
    pub _type: u16,
    class: u16,
}

fn wrap_name(name: &String) -> Vec<u8> {
    let split = name.split('.');
    let mut vec = Vec::new();
    for s in split {
        vec.push(s.len() as u8);
        vec.extend(s.bytes())
    }
    vec.push(0);
    vec
}

impl From<Question> for Vec<u8> {
    fn from(question: Question) -> Self {
        let mut result = Vec::new();
        result.extend(wrap_name(&question.name));
        result.extend(&question._type.to_be_bytes());
        result.extend(&question.class.to_be_bytes());
        result
    }
}

impl From<&mut Cursor<u8>> for Question {
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
}