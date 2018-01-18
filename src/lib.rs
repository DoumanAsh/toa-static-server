extern crate futures;
extern crate hyper;
extern crate memmap;
extern crate mime_guess;
extern crate deflate;
extern crate unicase;

use std::path;
use std::fs;
use std::io;
use std::time;

use unicase::Ascii;
use deflate::deflate_bytes;
use memmap::Mmap;
use futures::future::FutureResult;
use hyper::server::{NewService, Service, Request, Response};
use hyper::header;
use mime_guess::guess_mime_type;

fn cache_headers(stats: &fs::Metadata) -> (header::EntityTag, Option<header::LastModified>) {
    if let Ok(sys_modified) = stats.modified() {
        let modified = sys_modified.duration_since(time::UNIX_EPOCH).expect("Modified is earlier than time::UNIX_EPOCH!");
        let etag = header::EntityTag::strong(format!("{}.{}-{}", modified.as_secs(), modified.subsec_nanos(), stats.len()));
        let modified = time::SystemTime::now() - sys_modified.elapsed().expect("Failed to elapse metadata.modified()");
        (etag, Some(header::LastModified(modified.into())))
    }
    else {
        (header::EntityTag::strong(format!("{}", stats.len())), None)
    }
}

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
        let path = path.join(request_path);

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

    #[inline]
    ///Returns 304 response if cache is suitable
    fn cache_response(&self, req: &Request, expected_etag: &header::EntityTag) -> Option<Response> {
        let etags = match req.headers().get::<header::IfNoneMatch>() {
            Some(header) => {
                match *header {
                    header::IfNoneMatch::Items(ref etags) => etags,
                    _ => return None,
                }
            }
            None => return None,
        };

        //While we send Last-Modified.
        //Etag uses its and file size for its value.
        //Therefore if ETag matches then If-Unmodified-Since will match too
        for etag in etags {
            if expected_etag.strong_eq(&etag) {
                return Some(Response::new().with_status(hyper::StatusCode::NotModified))
            }
        }

        None
    }

    #[inline]
    ///Prepare response with file content.
    fn send_file(&self, req: &Request, stats: &fs::Metadata, file: &fs::File, mime: header::ContentType, etag: header::EntityTag, modified: Option<header::LastModified>) -> Response {
        let file = unsafe { Mmap::map(&file).unwrap() };
        let content = match req.headers().get::<header::AcceptEncoding>() {
            Some(header) => to_encoded_buffer(&file, header),
            None => to_buffer(&file)
        };

        let mut optional_headers = header::Headers::new();
        if let Some(modified) = modified {
            optional_headers.set(modified);
        }

        Response::new().with_status(hyper::StatusCode::Ok)
            .with_headers(optional_headers)
            .with_header(header::Server::new("Toa"))
            .with_header(header::Vary::Items(vec![Ascii::new("Accept-Encoding".to_owned())]))
            .with_header(header::ContentLength(stats.len()))
            .with_header(header::ContentEncoding(vec![header::Encoding::Deflate]))
            .with_header(header::CacheControl(vec![header::CacheDirective::Public]))
            .with_header(header::ETag(etag))
            .with_header(mime)
            .with_body(content)
    }
}

#[inline]
fn to_buffer(data: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(data.len());
    buffer.extend_from_slice(data);
    buffer
}

#[inline]
fn to_encoded_buffer(data: &[u8], header: &hyper::header::AcceptEncoding) -> Vec<u8> {
    for idx in 0..header.len() {
        if header[idx].item == header::Encoding::Deflate {
            return deflate_bytes(data);
        }
    }

    to_buffer(data)
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

        futures::future::ok(match self.get_file(&req.path()[1..]) {
            Some((Ok(file), mime)) => {
                let stats = match file.metadata() {
                    Ok(stats) => stats,
                    Err(error) => return futures::future::ok(self.internal_error(format!("{}", error))),
                };

                let (etag, modified) = cache_headers(&stats);

                if let Some(response) = self.cache_response(&req, &etag) {
                    response
                }
                else {
                    self.send_file(&req, &stats, &file, mime, etag, modified)
                }
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

#[cfg(test)]
mod tests {
    use super::hyper;
    use hyper::mime;
    #[test]
    fn to_buffer() {
        let data = [1, 2, 3, 4, 5, 6, 7, 100, 10, 20, 30];
        let result = super::to_buffer(&data);

        assert_eq!(data.len(), result.len());
        assert_eq!(&data, result.as_slice());
    }

    #[test]
    fn to_encoded_buffer_fail() {
        let data = [1, 2, 3, 4, 5, 6, 7, 100, 10, 20, 30];
        let header = hyper::header::AcceptEncoding(vec![]);
        let result = super::to_encoded_buffer(&data, &header);

        assert_eq!(data.len(), result.len());
        assert_eq!(&data, result.as_slice());
    }

    macro_rules! hyper_encoding {
        ($name:ident) => {
            hyper_encoding!($name, 10)
        };
        ($name:ident, $quality:expr) => {{
            hyper::header::QualityItem {
                item: hyper::header::Encoding::$name,
                quality: hyper::header::q($quality)
            }
        }};
    }

    #[test]
    fn to_encoded_buffer_ok() {
        let data = [1, 2, 3, 4, 5, 6, 7, 100, 10, 20, 30];
        let encodings = vec![hyper_encoding!(Gzip), hyper_encoding!(Deflate)];
        let header = hyper::header::AcceptEncoding(encodings);
        let result = super::to_encoded_buffer(&data, &header);

        assert_ne!(data.len(), result.len());
        assert_ne!(&data, result.as_slice());
    }

    use std::path;
    #[test]
    fn get_file_ok() {
        let current_file = file!();
        let current_dir = path::Path::new(current_file).parent().unwrap();
        let current_file = path::Path::new(current_file).file_name().unwrap().to_str().unwrap().to_string();

        let static_serve = super::StaticServe::new(current_dir.to_str().unwrap().to_string());

        let result = static_serve.get_file(&current_file);

        assert!(result.is_some());
        let (result_file, result_content) = result.unwrap();

        assert!(result_file.is_ok());
        assert_eq!((result_content.0).type_(), mime::TEXT);
        assert_eq!((result_content.0).subtype(), "x-rust");
    }

    #[test]
    fn get_file_fail() {
        let current_file = file!();
        let current_dir = path::Path::new(current_file).parent().unwrap();
        let current_file = path::Path::new(current_file).with_extension("lolka").file_name().unwrap().to_str().unwrap().to_string();

        let static_serve = super::StaticServe::new(current_dir.to_str().unwrap().to_string());

        let result = static_serve.get_file(&current_file);

        assert!(result.is_none());
    }
}
