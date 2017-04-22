use std::path;
use std::fs;
use std::io;

use ::memmap::Mmap;
use ::futures;
use ::futures::future::FutureResult;
use ::hyper;
use ::hyper::server::{Service, Request, Response};
use ::hyper::header;

#[derive(Clone)]
pub struct StaticServe(pub String);

fn get_static_file(root: &str, request_path: &str) -> Option<io::Result<fs::File>> {
    let path = path::Path::new(root);
    let path = path.join(&request_path[1..]);

    if path.is_file() {
        Some(fs::File::open(path))
    }
    else if path.is_dir() {
        let path = path.join("index.html");
        if path.is_file() {
            Some(fs::File::open(path))
        }
        else {
            None
        }
    }
    else {
        None
    }
}

impl Service for StaticServe {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let file = get_static_file(&self.0, req.path());

        futures::future::ok(match file {
            Some(Ok(file)) => {
                let stats = file.metadata().unwrap();
                let file = Mmap::open(&file, ::memmap::Protection::Read).unwrap();
                let mut content = Vec::new();
                content.extend_from_slice(unsafe { file.as_slice() });
                Response::new().with_status(hyper::StatusCode::Ok)
                               .with_header(header::ContentLength(stats.len()))
                               .with_body(content)

            },
            Some(Err(_)) => {
                Response::new().with_status(hyper::StatusCode::InternalServerError)
            },
            None => {
                Response::new().with_status(hyper::StatusCode::NotFound)
            }
        })
    }
}
