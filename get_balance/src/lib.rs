// Reference: https://github.com/serde-rs/json

#[macro_use] extern crate failure_derive;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate derive_getters;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate failure;
extern crate futures;
extern crate dotenv;
extern crate iron;
extern crate hyper_openssl;
extern crate chrono;
extern crate jsonrpc_core;
extern crate protobuf;
extern crate grpcio;

extern crate precision;
extern crate market_types;
extern crate client_service_proto;
extern crate hmac_authenticator_proto;

pub mod config;
pub mod iron_service;
pub mod jsonrpc_handlers;
pub mod model;


/*
 * extern crate serde;
 * extern crate serde_json;
 */

/*
use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Job = Box<FnBox + Send + 'static>;

#[derive(Serialize, Deserialize)]
pub struct GetBalance_Request {
    addresses: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetBalance_Reponse {
    address: String,
    utxos: Vec<utxo>,
}

struct utxo {
    txid: u64,
    index: u64,
    amount: f64,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    // Define new() method
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        // Create a channel between sender and receiver
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        // Create a vector "workers" that stores Worker instances 
        let mut workers = Vec::with_capacity(size);
        // Create a new Worker with the associated id and store it in "workers"
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender,
        }
    }
    // Define execute() method
    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();
                println!("Worker {} got a job; executing.", id);
                job.call_box();
            }
        });

        Worker {
            id,
            thread,
        }
    }
}
*/