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
        #[serde(rename = "myName")]
        my_name: String,
        #[serde(rename = "numGoodMessagesRecved")]
        num_good_messages_recved: u32,
        #[serde(rename = "numCommErrors")]
        num_comm_errors: u32,
        #[serde(rename = "numJSONRequestsRecved")]
        num_json_requests_recved: u32,
        #[serde(rename = "numJSONResponsesRecved")]
        num_json_responses_recved: u32,
        #[serde(rename = "numJSONRequestsSent")]
        num_json_requests_sent: u32,
        #[serde(rename = "numJSONResponsesSent")]
        num_json_responses_sent: u32,
    },
    Heartbeat,
    RequestNetstats,
    AdcReading {
        reading: u32,
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

    fn append<I>(&mut self, i: I)
        where I: IntoIterator<Item = u8>
    {
        self.buffer.extend(i);
    }

    fn add_message(&mut self, message: &Netmessage) {
        let s = serde_json::to_string(message)
            .unwrap_or_else(|e| panic!("Failed to serialize JSON: {}", e));
        self.append(s.bytes());
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

    // let mut message = Message::new();
    // message.add_message(&Netmessage::Netstats {
    // my_name: String::from("blah"),
    // num_good_messages_recved: 0,
    // num_comm_errors: 0,
    // num_json_requests_recved: 0,
    // num_json_responses_recved: 0,
    // num_json_requests_sent: 0,
    // num_json_responses_sent: 0,
    // });
    // let heartbeat = match message.finish() {
    // Ok(v) => v,
    // Err(e) => panic!("Failed to create heartbeat message: {:?}", e),
    // };

    let listener = TcpListener::bind("192.168.43.1:2000").unwrap();

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

        let json_iter = match stream.try_clone() {
            Ok(s) => serde_json::StreamDeserializer::<Netmessage, _>::new(s.bytes()),
            Err(e) => panic!("Unable to clone TCP stream: {}", e),
        };

        // Create a channel for sending back the Netmessages.
        let (sender, receiver) = channel();

        // Perform the JSON reading in a separate thread.
        thread::spawn(move || {
            for nmessage in json_iter {
                sender.send(nmessage).unwrap_or_else(|e| panic!("Failed to send nmessage: {}", e));
            }
        });

        // Create the time.
        let mut prevtime = time::Instant::now();

        // Create the heartbeat message.
        let mut message = Message::new();
        message.add_message(&Netmessage::RequestNetstats);
        let heartbeat = match message.finish() {
            Ok(v) => v,
            Err(e) => panic!("Failed to create heartbeat message: {:?}", e),
        };

        println!("##################");

        loop {
            // Perform a non-blocking read from the stream.
            match receiver.try_recv() {
                Ok(Ok(m)) => println!("JSON: {:?}", m),
                Ok(Err(e)) => panic!("Closing: Got invalid JSON: {}", e),
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
