extern crate futures;
extern crate hyper;
extern crate memmap;

use std::path;
use std::fs;
use std::io;

use memmap::Mmap;
use futures::future::FutureResult;
use hyper::server::{NewService, Service, Request, Response};
use hyper::header;

#[derive(Clone)]
///Hyper Static File Serve Service
pub struct StaticServe {
    ///Directory from where to serve
    pub root: String
}

impl StaticServe {
    pub fn new(root: String) -> StaticServe {
        StaticServe {
            root: root
        }
    }

    fn get_file(&self, request_path: &str) -> Option<io::Result<fs::File>> {
        let path = path::Path::new(&self.root);
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
}

impl Service for StaticServe {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let file = self.get_file(req.path());

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

impl NewService for StaticServe {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Self;

    fn new_service(&self) -> Result<Self::Instance, io::Error> {
        Ok(self.clone())
    }
}
