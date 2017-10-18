// #[derive(BartDisplay)] can cause unused extern crates warning:
#![allow(unused_extern_crates)]

use std::fmt;

use futures::{self, Future};
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;
use hyper;

use assets::StyleCss;
use web::Lookup;
use wiki_lookup::WikiLookup;

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
}

header! { (XIdentity, "X-Identity") => [String] }

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
pub struct Layout<'a, T: 'a + fmt::Display> {
    pub base: Option<&'a str>,
    pub title: &'a str,
    pub body: &'a T,
    pub style_css_checksum: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/error/404.html"]
struct NotFound;

#[derive(BartDisplay)]
#[template = "templates/error/500.html"]
struct InternalServerError;

pub struct Site {
    root: WikiLookup,
    trust_identity: bool,
}

impl Site {
    pub fn new(root: WikiLookup, trust_identity: bool) -> Site {
        Site { root, trust_identity }
    }

    fn not_found(base: Option<&str>) -> Response {
        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                base: base,
                title: "Not found",
                body: &NotFound,
                style_css_checksum: StyleCss::checksum(),
            }.to_string())
            .with_status(hyper::StatusCode::NotFound)
    }

    fn internal_server_error(base: Option<&str>, err: Box<::std::error::Error + Send + Sync>) -> Response {
        eprintln!("Internal Server Error:\n{:#?}", err);

        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                base,
                title: "Internal server error",
                body: &InternalServerError,
                style_css_checksum: StyleCss::checksum(),
            }.to_string())
            .with_status(hyper::StatusCode::InternalServerError)
    }
}

fn root_base_from_request_uri(path: &str) -> Option<String> {
    assert!(path.starts_with("/"));
    let slashes = path[1..].matches('/').count();

    match slashes {
        0 => None,
        n => Some(::std::iter::repeat("../").take(n).collect())
    }
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<futures::Future<Item = Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let (method, uri, _http_version, headers, body) = req.deconstruct();

        println!("{} {}", method, uri);

        let identity: Option<String> = match self.trust_identity {
            true => headers.get().map(|x: &XIdentity| x.to_string()),
            false => None,
        };

        let base = root_base_from_request_uri(uri.path());
        let base2 = base.clone(); // Bah, stupid clone

        Box::new(self.root.lookup(uri.path(), uri.query())
            .and_then(move |resource| match resource {
                Some(resource) => {
                    use hyper::Method::*;
                    match method {
                        Options => Box::new(futures::finished(resource.options())),
                        Head => resource.head(),
                        Get => resource.get(),
                        Put => resource.put(body, identity),
                        _ => Box::new(futures::finished(resource.method_not_allowed()))
                    }
                },
                None => Box::new(futures::finished(Self::not_found(base.as_ref().map(|x| &**x))))
            })
            .or_else(move |err| Ok(Self::internal_server_error(base2.as_ref().map(|x| &**x), err)))
        )
    }
}
