#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde;
extern crate serde_json;

use std::io::{Read, Write};
use std::sync::mpsc::{channel, TryRecvError};
use std::time;

#[derive(Debug, Deserialize, Serialize)]
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
        for _ in 0..8 {
            if self.crc & 0x8000 != 0 {
                self.crc ^= 0x1070 << 3;
            }
            self.crc <<= 1;
        }
    }

    fn finish(self) -> u8 {
        (self.crc >> 8) as u8
    }
}

struct Message {
    buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
enum MessageError {
    TooBig,
    TooSmall,
}

impl Message {
    fn new() -> Message {
        Message { buffer: Vec::new() }
    }

    fn add_byte(&mut self, byte: u8) {
        self.buffer.push(byte);
    }

    fn append<I>(&mut self, i: I)
        where I: IntoIterator<Item = u8>
    {
        self.buffer.extend(i);
    }

    fn finish(mut self) -> Result<Vec<u8>, MessageError> {
        if self.buffer.len() > 256 {
            Err(MessageError::TooBig)
        } else if self.buffer.len() == 0 {
            Err(MessageError::TooSmall)
        } else {
            let byte_len = (self.buffer.len() - 1) as u8;
            // Create CRC.
            let mut crc = Crc8::new();
            // Add the length byte to the CRC.
            crc.add_byte(byte_len);
            // Add the payload to the CRC.
            for b in &self.buffer {
                crc.add_byte(*b);
            }
            // Add the magic sequence, CRC, and length to the message payload.
            let mut v = vec![128, 37, 35, 46, crc.finish(), byte_len];
            // Add the entire payload.
            v.append(&mut self.buffer);
            Ok(v)
        }
    }
}

fn main() {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    let listener = TcpListener::bind("192.168.43.177:2000").unwrap();

    let nmessg = Netmessage::Netstats {
        myName: String::from("Sensor"),
        numGoodMessagesRecved: 0,
        numCommErrors: 0,
        numJSONRequestsRecved: 0,
        numJSONResponsesRecved: 0,
        numJSONRequestsSent: 0,
        numJSONResponsesSent: 0,
    };

    println!("Good netmsg: {}", serde_json::to_string(&nmessg).unwrap());

    fn handle_client(mut stream: TcpStream) {
        println!("New connection.");

        // The Wifly always sends this 7-byte sequence on connection.
        let mut initialbuff = [0u8; 7];
        stream.read_exact(&mut initialbuff).unwrap();
        if initialbuff != [42, 72, 69, 76, 76, 79, 42] {
            panic!("Didn't get magic values!");
        } else {
            println!("Got magic values, continuing.");
        }

        let mut json_iter = match stream.try_clone() {
            Ok(s) => s/*serde_json::StreamDeserializer::<Netmessage, _>::new(s.bytes())*/,
            Err(e) => panic!("Unable to clone TCP stream: {}", e),
        };

        // Create a channel for sending back the Netmessages.
        let (sender, receiver) = channel::<Result<Netmessage, TryRecvError>>();

        // Perform the JSON reading in a separate thread.
        thread::spawn(move || {
            let mut buff = [0u8; 100];
            loop {
                json_iter.read_exact(&mut buff).unwrap();
                for c in buff.iter() {
                    print!("{}", *c as char);
                }
                println!("");
            }
            // for nmessage in json_iter {
            // sender.send(nmessage).unwrap_or_else(|e| panic!("Failed to send nmessage: {}", e));
            // }
        });

        // Create the time.
        let mut prevtime = time::Instant::now();

        // Create the heartbeat message.
        let mut message = Message::new();
        message.add_byte(0xAA);
        let heartbeat = match message.finish() {
            Ok(v) => v,
            Err(e) => panic!("Failed to create heartbeat message: {:?}", e),
        };

        println!("Heartbeat: {:?}", heartbeat);

        loop {
            // Perform a non-blocking read from the stream.
            match receiver.try_recv() {
                Ok(Ok(m)) => println!("JSON: {:?}", m),
                Ok(Err(e)) => println!("Got invalid JSON: {}", e),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("Receiver disconnected."),
            }
            let currtime = time::Instant::now();
            if currtime - prevtime > time::Duration::from_secs(1) {
                prevtime = currtime;
                // Send heartbeat.
                stream.write_all(&heartbeat[..])
                    .unwrap_or_else(|e| panic!("Failed to send heartbeat: {}", e));
            }
        }
    }

    // Accept connections and process them, spawning a new thread for each one.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream));
            }
            Err(e) => {
                println!("Lost socket listener: {}", e);
            }
        }
    }
}
