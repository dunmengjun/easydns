mod header;
mod question;
mod answer;
mod basic;

use crate::cache::CacheRecord;
use crate::system::AnswerBuf;
use crate::cursor::Cursor;
use header::Header;
use question::Question;

const C_FACTOR: u8 = 192u8;
const DC_FACTOR: u16 = 16383u16;

pub use answer::{DnsAnswer, Ipv4Answer, FailureAnswer, SoaAnswer};

fn parse_name(cursor: &Cursor<u8>, name_vec: &mut Vec<u8>) {
    if cursor.peek() & C_FACTOR == C_FACTOR {
        let c_index = u16::from_be_bytes([cursor.take(), cursor.take()]);
        cursor.tmp_at((c_index & DC_FACTOR) as usize, |buf| {
            parse_name(buf, name_vec);
        })
    } else {
        let seg_len = cursor.take();
        if seg_len > 0 {
            let segment = cursor.take_slice(seg_len as usize);
            name_vec.push('.' as u8);
            name_vec.extend(segment);
            parse_name(cursor, name_vec)
        } else {
            cursor.take();
        }
    };
}

fn unzip_domain(cursor: &Cursor<u8>) -> String {
    let mut domain_vec = Vec::new();
    parse_name(cursor, &mut domain_vec);
    domain_vec.remove(0);
    String::from_utf8(domain_vec).unwrap()
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

