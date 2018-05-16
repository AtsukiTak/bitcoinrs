extern crate get_balance_lib;        // Load "lib.rs" from get_balance folder
extern crate env_logger;             // A logging implementation
extern crate grpcio;                 // Rust implementation of gRPC protocol
extern crate iron;                   // Concurrent Web Framework for Rust

// use get_balance::ThreadPool;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::fs::File;
use std::sync::Arc;
use grpcio::{ChannelBuilder, EnvBuilder};
use get_balance_lib::{config, iron_service, jsonrpc_handlers};

fn main() 
{
    let config = config::from_environment().unwrap();
    env_logger::init();

    let (listen, ssl, cs_grpc, ha_grpc) = config.consume();

    let grpc_env = Arc::new(EnvBuilder::new().build());
    let cs_channel = ChannelBuilder::new(grpc_env.clone()).connect(&cs_grpc[..]);
    let ha_channel = ChannelBuilder::new(grpc_env).connect(&ha_grpc[..]);
    
    let json_handler = jsonrpc_handlers::prepare(cs_channel);
    let json_rpc = iron_service::JsonRpc::new(json_handler, ha_channel);

    let _listening = if let Some(ssl) = ssl {
        let listening = iron::Iron::new(json_rpc).https(listen, ssl).unwrap();
        info!("JSONRPC HTTPS listening on {}", &listen);
        listening
    } else {
        let listening = iron::Iron::new(json_rpc).http(listen).unwrap();
        info!("JSONRPC HTTP listening on {}", &listen);
        listening
    };

    /*
    // Listen at localhost (127.0.0.1), port 8080
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    // Create a new thread pool with the maximum number of threads that it's able to contain
    let pool = ThreadPool::new(4);
    // When it gets an incoming stream, call handle_connection()
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }
    */
}

/*
// Function to read from the TcpStream
fn handle_connection(mut stream: TcpStream) 
{
    let mut buff = [0; 512];
    // Read from TcpStream and put them in buff
    stream.read(&mut buff).unwrap();

    // If the path is "127.0.0.1:8080/", render "hello.html". Otherwise, render "404.html" 
    let get = b"GET / HTTP/1.1\r\n";
    let (status_line, filename) = if buff.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };
        // Open a file (with filename)
        let mut file = File::open(filename).unwrap();
        // Read the contents of that file
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();
        // Write HTTP response status and body to stream 
        let response = format!("{}{}", status_line, body);
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
        /*
        // Convert bytes to strings and print them
        println!("Request: {}", String::from_utf8_lossy(&buff[..]));
        */
}
*/
