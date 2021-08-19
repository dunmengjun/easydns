use crate::cursor::Cursor;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    pub id: u16,
    pub flags: u16,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
    pub additional_count: u16,
}

impl From<&Header> for Vec<u8> {
    fn from(header: &Header) -> Self {
        let mut result = Vec::with_capacity(12);
        result.extend(&header.id.to_be_bytes());
        result.extend(&header.flags.to_be_bytes());
        result.extend(&header.question_count.to_be_bytes());
        result.extend(&header.answer_count.to_be_bytes());
        result.extend(&header.authority_count.to_be_bytes());
        result.extend(&header.additional_count.to_be_bytes());
        result
    }
}

impl From<&Cursor<u8>> for Header {
    fn from(cursor: &Cursor<u8>) -> Self {
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
}

impl Header {
    pub fn new() -> Self {
        Header {
            id: 0,
            flags: 0,
            question_count: 0,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        }
    }
}