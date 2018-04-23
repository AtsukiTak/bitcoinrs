extern crate get_balance;
use get_balance::ThreadPool;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::fs::File;

fn main() 
{
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
}

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
