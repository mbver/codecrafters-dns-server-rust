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
                let req = Message::parse(&buf[..size]).expect("Failed to parse DNS message");
                let mut header = Header::reply_from(&req.header);
                header.qdcount = 1;
                header.ancount = 1;

                let message = Message {
                    header,
                    questions: vec![Question {
                        name: String::from("codecrafters.io"),
                        qtype: 1,  // A record
                        qclass: 1, // IN class
                    }],
                    answers: vec![ResourceRecord::a_record(
                        "codecrafters.io",
                        60,
                        Ipv4Addr::new(8, 8, 8, 8),
                    )],
                };
                let mut response = Vec::new();
                message.write_to(&mut response);
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
