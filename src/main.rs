extern crate futures;
extern crate hyper;
extern crate memmap;

mod server;

use hyper::server::{Http};

fn main() {
    let addr = "127.0.0.1:3333".parse().unwrap();

    let server = Http::new().bind(&addr, || Ok(server::StaticServe(".".to_string()))).unwrap();
    println!("Start static server");
    server.run().unwrap();
}
