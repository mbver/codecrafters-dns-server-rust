pub struct Header {
    pub id: u16,
    pub qr: u8,     // 1 bit
    pub opcode: u8, // 4 bits
    pub aa: u8,     // 1 bit
    pub tc: u8,     // 1 bit
    pub rd: u8,     // 1 bit
    pub ra: u8,     // 1 bit
    pub z: u8,      // 3 bits
    pub rcode: u8,  // 4 bits
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

impl Header {
    pub fn stage_reply() -> Self {
        Self {
            id: 1234,
            qr: 1,
            opcode: 0,
            aa: 0,
            tc: 0,
            rd: 0,
            ra: 0,
            z: 0,
            rcode: 0,
            qdcount: 0,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    pub fn write_to(&self, out: &mut Vec<u8>) {
        let flags: u16 = ((self.qr as u16 & 0b1) << 15)
            | ((self.opcode as u16 & 0b1111) << 11)
            | ((self.aa as u16 & 0b1) << 10)
            | ((self.tc as u16 & 0b1) << 9)
            | ((self.rd as u16 & 0b1) << 8)
            | ((self.ra as u16 & 0b1) << 7)
            | ((self.z as u16 & 0b111) << 4)
            | (self.rcode as u16 & 0b1111);

        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(&flags.to_be_bytes());
        out.extend_from_slice(&self.qdcount.to_be_bytes());
        out.extend_from_slice(&self.ancount.to_be_bytes());
        out.extend_from_slice(&self.nscount.to_be_bytes());
        out.extend_from_slice(&self.arcount.to_be_bytes());
    }
}

pub struct Message {
    pub header: Header,
    pub questions: Vec<Question>,
}

impl Message {
    pub fn write_to(&self, out: &mut Vec<u8>) {
        self.header.write_to(out);
        for question in &self.questions {
            question.write_to(out);
        }
    }
}

pub struct Question {
    pub name: String, // e.g. "codecrafters.io"
    pub qtype: u16,   // 1 = A
    pub qclass: u16,  // 1 = IN
}

impl Question {
    pub fn write_to(&self, out: &mut Vec<u8>) {
        for label in self.name.split('.').filter(|s| !s.is_empty()) {
            let len = u8::try_from(label.len()).expect("DNS label exceeds 63 bytes");
            out.push(len);
            out.extend_from_slice(label.as_bytes());
        }
        out.push(0); // null terminator

        out.extend_from_slice(&self.qtype.to_be_bytes());
        out.extend_from_slice(&self.qclass.to_be_bytes());
    }
}
