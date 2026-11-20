use std::net::{Ipv4Addr, UdpSocket};

use crate::message::{Header, Message, Question, ResourceRecord};

mod message;

fn main() {
    // when the udp socket is connected, i.e, set the peer address in the kernel filter
    // we can use recv and send instead of recv_from and send_to
    // however, packets will be filter by that peer address and send_to another peer will fail
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];
    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);
                let packet = &buf[..size];
                let req_header = Header::parse(packet).expect("Failed to parse header");

                let mut offset = 12;
                let mut questions = Vec::with_capacity(req_header.qdcount as usize);
                for _ in 0..req_header.qdcount {
                    questions.push(
                        Question::parse(packet, &mut offset).expect("Failed to parse question"),
                    );
                }

                let mut reply_header = Header::reply_from(&req_header);
                reply_header.qdcount = questions.len() as u16;

                let answers: Vec<ResourceRecord> = questions
                    .iter()
                    .map(|q| ResourceRecord::a_record(&q.name, 60, Ipv4Addr::new(8, 8, 8, 8)))
                    .collect();
                reply_header.ancount = answers.len() as u16;

                let reply = Message {
                    header: reply_header,
                    questions,
                    answers,
                };
                let mut response = Vec::new();
                reply.write_to(&mut response);
                udp_socket
                    .send_to(&response, source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
