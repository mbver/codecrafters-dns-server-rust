use std::{
    env,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::Duration,
};

use crate::message::{Header, Message, ResourceRecord};

mod message;

fn parse_resolver_arg() -> Option<SocketAddr> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--resolver" {
            let addr_str = args.next().expect("Missing value after --resolver");
            let addr: SocketAddr = addr_str.parse().expect("Invalid --resolver socket address");
            return Some(addr);
        }
    }
    None
}

fn build_local_reply(req: &Message) -> Message {
    let mut reply_header = Header::reply_from(&req.header);
    reply_header.qdcount = req.questions.len() as u16;

    let answers: Vec<ResourceRecord> = req
        .questions
        .iter()
        .map(|q| ResourceRecord::a_record(&q.name, 60, Ipv4Addr::new(8, 8, 8, 8)))
        .collect();
    reply_header.ancount = answers.len() as u16;

    Message {
        header: reply_header,
        questions: req.questions.clone(),
        answers,
    }
}

fn forward_query(req: &Message, resolver_addr: SocketAddr) -> Result<Message, &'static str> {
    let upstream = UdpSocket::bind("0.0.0.0:0").map_err(|_| "Failed to bind upstream socket")?;
    upstream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "Failed to set upstream read timeout")?;

    let mut merged_answers: Vec<ResourceRecord> = Vec::new();
    let mut query_buf = Vec::with_capacity(512);
    let mut recv_buf = [0u8; 4096];

    for (idx, question) in req.questions.iter().enumerate() {
        let sub_header = Header {
            id: (idx as u16).wrapping_add(1),
            qr: 0,
            opcode: req.header.opcode,
            aa: 0,
            tc: 0,
            rd: req.header.rd,
            ra: 0,
            z: 0,
            rcode: 0,
            qdcount: 1,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        };

        let sub_query = Message {
            header: sub_header,
            questions: vec![question.clone()],
            answers: Vec::new(),
        };

        query_buf.clear();
        sub_query.write_to(&mut query_buf);

        upstream
            .send_to(&query_buf, resolver_addr)
            .map_err(|_| "Failed to send sub-query to upstream resolver")?;

        let (size, _) = upstream
            .recv_from(&mut recv_buf)
            .map_err(|_| "Failed to receive response from upstream resolver")?;

        let upstream_reply = Message::parse(&recv_buf[..size])?;
        merged_answers.extend(upstream_reply.answers);
    }

    let mut reply_header = Header::reply_from(&req.header);
    reply_header.qdcount = req.questions.len() as u16;
    reply_header.ancount = merged_answers.len() as u16;

    Ok(Message {
        header: reply_header,
        questions: req.questions.clone(),
        answers: merged_answers,
    })
}

fn main() {
    let upstream_addr = parse_resolver_arg();
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0u8; 4096];
    let mut response_buf = Vec::with_capacity(512);

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);

                let req =
                    Message::parse(&buf[..size]).expect("Failed to parse incoming DNS message");
                let reply = match upstream_addr {
                    Some(resolver_addr) => {
                        forward_query(&req, resolver_addr).expect("Failed to forward DNS query")
                    }
                    None => build_local_reply(&req),
                };

                response_buf.clear();
                reply.write_to(&mut response_buf);

                udp_socket
                    .send_to(&response_buf, source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
