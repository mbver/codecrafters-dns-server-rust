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
    pub fn parse(buf: &[u8]) -> Result<Self, &'static str> {
        if buf.len() < 12 {
            return Err("DNS header must be at least 12 bytes");
        }

        let id = u16::from_be_bytes([buf[0], buf[1]]);
        let flags = u16::from_be_bytes([buf[2], buf[3]]);
        let qdcount = u16::from_be_bytes([buf[4], buf[5]]);
        let ancount = u16::from_be_bytes([buf[6], buf[7]]);
        let nscount = u16::from_be_bytes([buf[8], buf[9]]);
        let arcount = u16::from_be_bytes([buf[10], buf[11]]);

        Ok(Self {
            id,
            qr: ((flags >> 15) & 0b1) as u8,
            opcode: ((flags >> 11) & 0b1111) as u8,
            aa: ((flags >> 10) & 0b1) as u8,
            tc: ((flags >> 9) & 0b1) as u8,
            rd: ((flags >> 8) & 0b1) as u8,
            ra: ((flags >> 7) & 0b1) as u8,
            z: ((flags >> 4) & 0b111) as u8,
            rcode: (flags & 0b1111) as u8,
            qdcount,
            ancount,
            nscount,
            arcount,
        })
    }

    pub fn reply_from(req: &Header) -> Self {
        let rcode = if req.opcode == 0 { 0 } else { 4 };

        Self {
            id: req.id,
            qr: 1,
            opcode: req.opcode,
            aa: 0,
            tc: 0,
            rd: req.rd,
            ra: 0,
            z: 0,
            rcode,
            qdcount: 0,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }
}

pub struct Message {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<ResourceRecord>,
}

impl Message {
    pub fn write_to(&self, out: &mut Vec<u8>) {
        self.header.write_to(out);
        for question in &self.questions {
            question.write_to(out);
        }
        for r in &self.answers {
            r.write_to(out);
        }
    }
}

pub struct Question {
    pub name: String, // e.g. "codecrafters.io"
    pub qtype: u16,   // 1 = A
    pub qclass: u16,  // 1 = IN
}

impl Question {
    pub fn parse(buf: &[u8], offset: &mut usize) -> Result<Self, &'static str> {
        let name = read_domain_name(buf, offset)?;

        if *offset + 4 > buf.len() {
            return Err("Buffer too short for question type and class");
        }

        let qtype = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]);
        let qclass = u16::from_be_bytes([buf[*offset + 2], buf[*offset + 3]]);
        *offset += 4;

        Ok(Self {
            name,
            qtype,
            qclass,
        })
    }
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

use std::net::Ipv4Addr;

pub struct ResourceRecord {
    pub name: String,
    pub rtype: u16,
    pub class: u16,
    pub ttl: u32,
    pub rdata: Vec<u8>,
}

impl ResourceRecord {
    pub fn a_record(name: impl Into<String>, ttl: u32, ip: Ipv4Addr) -> Self {
        Self {
            name: name.into(),
            rtype: 1, // A
            class: 1, // IN
            ttl,
            rdata: ip.octets().to_vec(),
        }
    }

    pub fn write_to(&self, out: &mut Vec<u8>) {
        write_domain_name(&self.name, out);
        out.extend_from_slice(&self.rtype.to_be_bytes());
        out.extend_from_slice(&self.class.to_be_bytes());
        out.extend_from_slice(&self.ttl.to_be_bytes());

        let rdlength = u16::try_from(self.rdata.len()).expect("RDATA exceeds u16::MAX");
        out.extend_from_slice(&rdlength.to_be_bytes());
        out.extend_from_slice(&self.rdata);
    }
}

pub fn write_domain_name(name: &str, out: &mut Vec<u8>) {
    for label in name.split('.').filter(|s| !s.is_empty()) {
        let len = u8::try_from(label.len()).expect("DNS label exceeds 63 bytes");
        out.push(len);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

fn read_domain_name(buf: &[u8], offset: &mut usize) -> Result<String, &'static str> {
    let mut labels: Vec<String> = Vec::new();
    let mut cursor = *offset;
    let mut jumped = false;
    let mut jumps = 0usize;
    const MAX_JUMPS: usize = 64;

    loop {
        if cursor >= buf.len() {
            return Err("Unexpected end of buffer while reading domain name");
        }

        let len = buf[cursor];

        // Compression pointer: top 2 bits are 11
        if (len & 0b1100_0000) == 0b1100_0000 {
            if cursor + 1 >= buf.len() {
                return Err("Truncated compression pointer");
            }

            let ptr_offset = (((len & 0b0011_1111) as usize) << 8) | (buf[cursor + 1] as usize);
            if ptr_offset >= buf.len() {
                return Err("Compression pointer out of bounds");
            }

            // Advance the caller's stream offset past the 2-byte pointer only on the first jump
            if !jumped {
                *offset = cursor + 2;
                jumped = true;
            }

            jumps += 1;
            if jumps > MAX_JUMPS {
                return Err("Too many compression pointer jumps (possible loop)");
            }

            cursor = ptr_offset;
            continue;
        }

        // Standard label length (top 2 bits are 00)
        if (len & 0b1100_0000) != 0 {
            return Err("Invalid label length prefix");
        }

        cursor += 1;

        if len == 0 {
            if !jumped {
                *offset = cursor;
            }
            break;
        }

        let label_len = len as usize;
        if cursor + label_len > buf.len() {
            return Err("Label extends past end of buffer");
        }

        let label = std::str::from_utf8(&buf[cursor..cursor + label_len])
            .map_err(|_| "Invalid UTF-8 in domain label")?;
        labels.push(label.to_string());
        cursor += label_len;
    }

    Ok(labels.join("."))
}
