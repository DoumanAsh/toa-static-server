extern crate futures;
extern crate hyper;
extern crate memmap;
extern crate mime_guess;

use std::path;
use std::fs;
use std::io;

use memmap::Mmap;
use futures::future::FutureResult;
use hyper::server::{NewService, Service, Request, Response};
use hyper::header;
use mime_guess::guess_mime_type;

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

    fn get_file(&self, request_path: &str) -> Option<(io::Result<fs::File>, header::ContentType)> {
        let path = path::Path::new(&self.root);
        let path = path.join(&request_path[1..]);

        if path.is_file() {
            let mime = header::ContentType(guess_mime_type(&path));
            Some((fs::File::open(path), mime))
        }
        else if path.is_dir() {
            let path = path.join("index.html");
            if path.is_file() {
                Some((fs::File::open(path), header::ContentType::html()))
            }
            else {
                None
            }
        }
        else {
            None
        }
    }

    #[inline]
    fn method_not_allowed(&self) -> Response {
        const NOT_ALLOWED: &'static [u8] = b"<h1>Method is not allowed</h1>";
        Response::new().with_status(hyper::StatusCode::MethodNotAllowed)
                       .with_body(NOT_ALLOWED)
    }

    #[inline]
    fn internal_error(&self, msg: String) -> Response {
        Response::new().with_status(hyper::StatusCode::InternalServerError)
                       .with_body(msg)
    }

    #[inline]
    fn not_found(&self) -> Response {
        const NOT_FOUND: &'static [u8] = b"<h1>Page not found</h1>";
        Response::new().with_status(hyper::StatusCode::NotFound)
                       .with_body(NOT_FOUND)
    }
}

impl Service for StaticServe {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        if req.method() != &hyper::Method::Get {
            return futures::future::ok(self.method_not_allowed());
        }

        futures::future::ok(match self.get_file(req.path()) {
            Some((Ok(file), mime)) => {
                let stats = file.metadata().unwrap();

                let file = Mmap::open(&file, ::memmap::Protection::Read).unwrap();
                let mut content = Vec::new();
                content.extend_from_slice(unsafe { file.as_slice() });
                Response::new().with_status(hyper::StatusCode::Ok)
                               .with_header(header::ContentLength(stats.len()))
                               .with_header(mime)
                               .with_body(content)

            },
            Some((Err(error), _)) => self.internal_error(format!("{}", error)),
            None => self.not_found()
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
