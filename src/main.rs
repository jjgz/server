#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde;
extern crate serde_json;

use std::io::{Read, Write};

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
enum Netmessage {
    Netstats {
        myName: String,
        numGoodMessagesRecved: u32,
        numCommErrors: u32,
        numJSONRequestsRecved: u32,
        numJSONResponsesRecved: u32,
        numJSONRequestsSent: u32,
        numJSONResponsesSent: u32,
    },
}

struct Crc8 {
    crc: u16,
}

impl Crc8 {
    fn new() -> Crc8 {
        Crc8 { crc: 0 }
    }

    fn add_byte(&mut self, byte: u8) {
        self.crc ^= (byte as u16) << 8;
        for i in (1..9).rev() {
            if self.crc & 0x8000 != 0 {
                self.crc ^= 0x1070 << 3;
            }
            self.crc <<= 1;
        }
    }

    fn end(self) -> u8 {
        (self.crc >> 8) as u8
    }
}

fn main() {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    let listener = TcpListener::bind("192.168.43.177:2000").unwrap();

    fn handle_client(mut stream: TcpStream) {
        println!("New connection.");
        let mut initialbuff = [0u8; 7];
        let magic_start = [128u8, 37, 35, 46];
        let buff = [0u8, 'c' as u8];
        stream.read_exact(&mut initialbuff).unwrap();
        if initialbuff != [42, 72, 69, 76, 76, 79, 42] {
            panic!("Didn't get magic values!");
        } else {
            println!("Got magic values, continuing.");
        }
        let mut readstuff = [0u8; 1];
        loop {
            stream.write(&buff)
                .unwrap_or_else(|e| panic!("Writing to TCP stream failed: {}", e));
            // match serde_json::from_reader::<_, Netmessage>(&mut stream) {
            // Ok(netmessage) => {
            // println!("Packet: {:?}", netmessage);
            // }
            // Err(e) => println!("Failed to read JSON from stream: {}", e),
            // }
            stream.read_exact(&mut readstuff).unwrap();
            println!("Got: {:?}", String::from_utf8(readstuff.to_vec()));
        }
    }

    // accept connections and process them, spawning a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    // connection succeeded
                    handle_client(stream)
                });
            }
            Err(e) => {
                println!("Connection dropped: {}", e);
            }
        }
    }

    // close the socket server
    drop(listener);
}
