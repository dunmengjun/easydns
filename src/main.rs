use std::net::UdpSocket;
use rand::random;

struct Header {
    identifier: u16,
    flags: u16,
    question_count: u16,
    answer_count: u16,
    authority_count: u16,
    additional_count: u16,
}

impl Header {
    fn new() -> Self {
        Header {
            identifier: random(),
            flags: 1, //00000001 00000000, 2 -> 00000002 00000000
            question_count: 256, //00000000 00000001, 512 -> 00000000 00000002
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        }
    }
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts((self as *const Self) as *const u8, 12)
        }
    }
}

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
    fn as_bytes(&mut self) -> &[u8] {
        let x = self._type.to_be_bytes();
        let y = self.class.to_be_bytes();
        self.name.push(x[0]);
        self.name.push(x[1]);
        self.name.push(y[0]);
        self.name.push(y[1]);
        self.name.as_slice()
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
        for c in self.header.as_bytes() {
            bytes.push(c.clone());
        }
        self.questions.iter_mut().for_each(|question| {
            for c in question.as_bytes() {
                bytes.push(c.clone());
            }
        });
        bytes
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
    println!("recv start");
    match socket.recv_from(&mut buf) {
        Ok(result) => {
            println!("size: {}, addr: {}", result.0, result.1);
            println!("recv {:?}", &buf[0..32]);
        }
        Err(e) => println!("res error {:?}", e)
    }
    println!("recv end");
}
