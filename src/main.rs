#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde;
extern crate serde_json;

use std::io::{Read, Write};
use std::sync::mpsc::{channel, TryRecvError, Sender};
use std::time;
use std::sync::{Mutex, Arc};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
enum Netmessage {
    ReqName,
    NameJosh,
    NameGeordon,
    NameZach,
    NameJoe,
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
    ReqNetstats,
    /// Always requested by Geordon; the tick sent is the oldest tick for which movement is unknown.
    ReqJoeMovement(u32),
    /// Sends the movement data for the tick requested.
    JoeMovement { tick: u32, mov: f32, turn: f32 },
    /// Always requested by Geordon; the tick sent is the oldest tick for which movement is unknown.
    ReqJoshMovement(u32),
    /// Sends the movement data for the tick requested.
    JoshMovement { tick: u32, mov: f32, turn: f32 },
    /// Always requested by Zach.
    ReqStopped,
    /// Always sent by Josh.
    Stopped(u32),
    /// Sent to Zach.
    GeoReqGrabbed,
    /// Send to Geordon.
    GrabbedGeo(bool),
    /// Sent to Zach.
    JoshReqGrabbed,
    /// Send to Josh.
    GrabbedJosh(bool),
    /// Send to Josh.
    ReqStarted,
    /// Send to Zach.
    Started(bool),
    JoshReqWorld,
    WallJosh {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    BarrierJosh {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    EdgeJosh {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
    },
    JoeReqWorld,
    WallJoe {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    BarrierJoe {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    EdgeJoe {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
    },
    ZachReqWorld,
    WallZach {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    BarrierZach {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    EdgeZach {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
    },
    SrvReqWorld,
    WallSrv {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    BarrierSrv {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
        sx: f32,
        sy: f32,
        /// If it is open, then it doesn't necessarily end there.
        xopen: bool,
        /// If it is open, then it doesn't necessarily end there.
        yopen: bool,
    },
    EdgeSrv {
        /// This unique ID specifies which previous line to replace.
        uid: u32,
        x: f32,
        y: f32,
    },
}

impl Netmessage {
    fn bot_name(&self) -> String {
        String::from(match *self {
            Netmessage::NameGeordon => "Geordon",
            Netmessage::NameJoe => "Joe",
            Netmessage::NameJosh => "Josh",
            Netmessage::NameZach => "Zach",
            _ => "Unnamed",
        })
    }
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

    fn from_netmessage(nm: &Netmessage) -> Vec<u8> {
        let mut message = Message::new();
        message.add_message(nm);
        message.finish()
            .expect(&format!("Error: Failed to create message from netmessage: {:?}", nm))
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

type PSender = Arc<Mutex<Option<Sender<Netmessage>>>>;

fn route_message(ps: &PSender, m: Netmessage) {
    match *ps.lock().unwrap() {
        Some(ref c) => {
            match c.send(m.clone()) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: Failed to send message on disconnected channel: {}",
                             e);
                    println!("Message: {:?}", m);
                }
            }
        }
        None => {
            println!("Warning: Attempted to send a request before the module connected.");
            println!("Message: {:?}", m);
        }
    }
}

fn main() {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    let geordon_sender = Arc::new(Mutex::new(None));
    let josh_sender = Arc::new(Mutex::new(None));
    let joe_sender = Arc::new(Mutex::new(None));
    let zach_sender = Arc::new(Mutex::new(None));

    fn handle_client(mut stream: TcpStream,
                     geordon_sender: PSender,
                     josh_sender: PSender,
                     joe_sender: PSender,
                     zach_sender: PSender) {
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
                sender.send(nmessage)
                    .unwrap_or_else(|e| panic!("Failed to send nmessage: {}", e));
            }
        });

        // Create the Heartbeat message.
        let heartbeat = Message::from_netmessage(&Netmessage::Heartbeat);
        // Create the RequestNetstats message.
        let request_netstats = Message::from_netmessage(&Netmessage::ReqNetstats);
        // Create the RequestName message.
        let request_name = Message::from_netmessage(&Netmessage::ReqName);

        println!("##################");

        // Create the time.
        let mut prev_heartbeat = time::Instant::now();
        let mut prev_request_netstats = time::Instant::now();
        let mut prev_req_name = time::Instant::now();

        let mut self_receiver = None;
        let mut self_name = None;

        loop {
            // Perform a non-blocking read from the stream.
            match receiver.try_recv() {
                Ok(Ok(m)) => {
                    match m {
                        Netmessage::NameGeordon => {
                            let chan = channel::<Netmessage>();
                            self_receiver = Some(chan.1);
                            *geordon_sender.lock().unwrap() = Some(chan.0);
                            self_name = Some(Netmessage::NameGeordon);
                            println!("Geordon robot identified.");
                        }
                        Netmessage::NameJoe => {
                            let chan = channel();
                            self_receiver = Some(chan.1);
                            *joe_sender.lock().unwrap() = Some(chan.0);
                            self_name = Some(Netmessage::NameJoe);
                            println!("Joe robot identified.");
                        }
                        Netmessage::NameJosh => {
                            let chan = channel();
                            self_receiver = Some(chan.1);
                            *josh_sender.lock().unwrap() = Some(chan.0);
                            self_name = Some(Netmessage::NameJosh);
                            println!("It's Josh bitch.");
                        }
                        Netmessage::NameZach => {
                            let chan = channel();
                            self_receiver = Some(chan.1);
                            *zach_sender.lock().unwrap() = Some(chan.0);
                            self_name = Some(Netmessage::NameZach);
                            println!("Zach robot identified.");
                        }
                        m @ Netmessage::ReqJoeMovement(..) |
                        m @ Netmessage::WallJoe { .. } |
                        m @ Netmessage::BarrierJoe { .. } |
                        m @ Netmessage::EdgeJoe { .. } |
                        m @ Netmessage::Started(..) => {
                            route_message(&joe_sender, m);
                        }
                        m @ Netmessage::ReqJoshMovement(..) |
                        m @ Netmessage::ReqStopped |
                        m @ Netmessage::GrabbedJosh(..) |
                        m @ Netmessage::WallJosh { .. } |
                        m @ Netmessage::BarrierJosh { .. } |
                        m @ Netmessage::EdgeJosh { .. } |
                        m @ Netmessage::ReqStarted => {
                            route_message(&josh_sender, m);
                        }
                        m @ Netmessage::JoeMovement { .. } |
                        m @ Netmessage::JoshMovement { .. } |
                        m @ Netmessage::GrabbedGeo(..) |
                        m @ Netmessage::JoshReqWorld |
                        m @ Netmessage::JoeReqWorld |
                        m @ Netmessage::ZachReqWorld => {
                            route_message(&geordon_sender, m);
                        }
                        m @ Netmessage::GeoReqGrabbed |
                        m @ Netmessage::JoshReqGrabbed |
                        m @ Netmessage::Stopped(..) |
                        m @ Netmessage::WallZach { .. } |
                        m @ Netmessage::BarrierZach { .. } |
                        m @ Netmessage::EdgeZach { .. } => {
                            route_message(&zach_sender, m);
                        }
                        m => {
                            println!("{}: {:?}",
                                     self_name.as_ref()
                                         .map(|o| o.bot_name())
                                         .unwrap_or(String::from("Unnamed")),
                                     m);
                        }
                    }
                }
                Ok(Err(e)) => panic!("Closing: Got invalid JSON: {}", e),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("Receiver disconnected."),
            }
            // Perform a non-blocking read from the stream.
            if let Some(ref sr) = self_receiver {
                match sr.try_recv() {
                    Ok(m) => {
                        stream.write_all(&Message::from_netmessage(&m)[..])
                            .unwrap_or_else(|e| {
                                panic!("Failed to route message from other bot: {}", e)
                            });
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Self receiver disconnected."),
                }
            }
            let currtime = time::Instant::now();
            if currtime - prev_heartbeat > time::Duration::from_secs(1) {
                prev_heartbeat = currtime;
                // Send Heartbeat.
                stream.write_all(&heartbeat[..])
                    .unwrap_or_else(|e| panic!("Failed to send Heartbeat: {}", e));
            }
            if currtime - prev_request_netstats > time::Duration::from_secs(5) {
                prev_request_netstats = currtime;
                // Send RequestNetstats.
                stream.write_all(&request_netstats[..])
                    .unwrap_or_else(|e| panic!("Failed to send RequestNetstats: {}", e));
            }
            if self_receiver.is_none() &&
               currtime - prev_req_name > time::Duration::from_millis(100) {
                prev_req_name = currtime;
                // Send RequestNetstats.
                stream.write_all(&request_name[..])
                    .unwrap_or_else(|e| panic!("Failed to send ReqName: {}", e));
            }
        }
    }

    let listener = TcpListener::bind("192.168.43.1:2000").unwrap();

    // Accept connections and process them, spawning a new thread for each one.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let (geordon_sender, josh_sender, joe_sender, zach_sender) =
                    (geordon_sender.clone(),
                     josh_sender.clone(),
                     joe_sender.clone(),
                     zach_sender.clone());
                thread::spawn(move || {
                    handle_client(stream, geordon_sender, josh_sender, joe_sender, zach_sender)
                });
            }
            Err(e) => {
                println!("Lost socket listener: {}", e);
            }
        }
    }
}
