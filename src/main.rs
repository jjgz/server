#![feature(proc_macro)]

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate glium;
extern crate glowygraph;

mod net;
mod render;

use std::sync::{Mutex, Arc};
use std::sync::mpsc::channel;
use std::thread;
use std::net::TcpListener;

use std::env::args;

fn main() {
    let geordon_sender = Arc::new(Mutex::new(None));
    let josh_sender = Arc::new(Mutex::new(None));
    let joe_sender = Arc::new(Mutex::new(None));
    let zach_sender = Arc::new(Mutex::new(None));

    let (server_sender, server_receiver) = channel();

    let bindaddress = args()
        .nth(1)
        .unwrap_or_else(|| panic!("Error: Pass an address in the format \"ip:port\" to bind to."));

    let listener = TcpListener::bind::<&str>(&bindaddress).unwrap();

    thread::spawn(move || render::render(server_receiver));

    // Accept connections and process them, spawning a new thread for each one.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let (geordon_sender, josh_sender, joe_sender, zach_sender, server_sender) =
                    (geordon_sender.clone(),
                     josh_sender.clone(),
                     joe_sender.clone(),
                     zach_sender.clone(),
                     server_sender.clone());
                thread::spawn(move || {
                    net::handle_client(stream,
                                       geordon_sender,
                                       josh_sender,
                                       joe_sender,
                                       zach_sender,
                                       server_sender);
                });
            }
            Err(e) => {
                println!("Lost socket listener: {}", e);
            }
        }
    }
}
