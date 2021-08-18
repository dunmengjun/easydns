use crate::cursor::Cursor;

const QUERY_ONLY_RECURSIVELY: u16 = 0x0100;
const QUERY_RECURSIVELY_AD: u16 = 0x0120;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    pub id: u16,
    pub flags: u16,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
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
    pub fn from(cursor: &mut Cursor<u8>) -> Self {
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