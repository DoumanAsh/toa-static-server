extern crate hyper;

extern crate hyper_static;

use hyper_static::{StaticServe};

use hyper::server::{Http};

use std::net;
use std::process::exit;

#[macro_use]
mod utils;
mod cli;

fn run() -> Result<i32, String> {
    let args = cli::Args::new();
    let port = args.get_u16("port")?;
    let dir = args.get_string("dir");
    let addr = net::SocketAddr::new(net::IpAddr::V4(net::Ipv4Addr::new(127, 0, 0, 1)), port);

    println!("Start static server on port {}. Serve directory='{}'", port, &dir);
    let server = Http::new().bind(&addr, StaticServe::new(dir)).unwrap();
    server.run().unwrap();

    Ok(0)
}

fn main() {
    exit(match run() {
        Ok(res) => res,
        Err(error) => {
            error_println!("{}", error);
            1
        }
    });
}
