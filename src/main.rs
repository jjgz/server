extern crate serde_json;
extern crate rnet;

mod net;

use std::sync::{Mutex, Arc};
use std::thread;
use std::net::TcpListener;

use std::env::args;

fn main() {
    let geordon_sender = Arc::new(Mutex::new(None));
    let josh_sender = Arc::new(Mutex::new(None));
    let joe_sender = Arc::new(Mutex::new(None));
    let zach_sender = Arc::new(Mutex::new(None));
    let debug_geordon_sender = Arc::new(Mutex::new(None));
    let debug_josh_sender = Arc::new(Mutex::new(None));
    let debug_joe_sender = Arc::new(Mutex::new(None));
    let debug_zach_sender = Arc::new(Mutex::new(None));

    let bindaddress = args()
        .nth(1)
        .unwrap_or_else(|| panic!("Error: Pass an address in the format \"ip:port\" to bind to."));

    let listener = TcpListener::bind::<&str>(&bindaddress).unwrap();

    // Accept connections and process them, spawning a new thread for each one.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let (geordon_sender,
                     josh_sender,
                     joe_sender,
                     zach_sender,
                     debug_geordon_sender,
                     debug_josh_sender,
                     debug_joe_sender,
                     debug_zach_sender) = (geordon_sender.clone(),
                                           josh_sender.clone(),
                                           joe_sender.clone(),
                                           zach_sender.clone(),
                                           debug_geordon_sender.clone(),
                                           debug_josh_sender.clone(),
                                           debug_joe_sender.clone(),
                                           debug_zach_sender.clone());
                thread::spawn(move || {
                    net::handle_client(stream,
                                       geordon_sender,
                                       josh_sender,
                                       joe_sender,
                                       zach_sender,
                                       debug_geordon_sender,
                                       debug_josh_sender,
                                       debug_joe_sender,
                                       debug_zach_sender);
                });
            }
            Err(e) => {
                println!("Lost socket listener: {}", e);
            }
        }
    }
}
