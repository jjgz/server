#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate serde;
extern crate serde_json;

mod net;
use std::sync::{Mutex, Arc};

fn main() {
    use std::net::TcpListener;
    use std::thread;

    let geordon_sender = Arc::new(Mutex::new(None));
    let josh_sender = Arc::new(Mutex::new(None));
    let joe_sender = Arc::new(Mutex::new(None));
    let zach_sender = Arc::new(Mutex::new(None));

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
                    net::handle_client(stream, geordon_sender, josh_sender, joe_sender, zach_sender)
                });
            }
            Err(e) => {
                println!("Lost socket listener: {}", e);
            }
        }
    }
}
