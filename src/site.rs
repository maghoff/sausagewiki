// #[derive(BartDisplay)] can cause unused extern crates warning:
#![allow(unused_extern_crates)]

use std::fmt;

use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;

use assets::StyleCss;
use web::Lookup;
use wiki_lookup::WikiLookup;

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
}

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
pub struct Layout<'a, T: 'a + fmt::Display> {
    pub title: &'a str,
    pub body: &'a T,
    pub style_css_checksum: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/404.html"]
struct NotFound;

#[derive(BartDisplay)]
#[template = "templates/500.html"]
struct InternalServerError;

pub struct Site {
    root: WikiLookup,
}

impl Site {
    pub fn new(root: WikiLookup) -> Site {
        Site { root }
    }

    fn not_found() -> Response {
        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Not found",
                body: &NotFound,
                style_css_checksum: StyleCss::checksum(),
            }.to_string())
            .with_status(hyper::StatusCode::NotFound)
    }

    fn internal_server_error(err: Box<::std::error::Error + Send + Sync>) -> Response {
        eprintln!("Internal Server Error:\n{:#?}", err);

        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Internal server error",
                body: &InternalServerError,
                style_css_checksum: StyleCss::checksum(),
            }.to_string())
            .with_status(hyper::StatusCode::InternalServerError)
    }
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let (method, uri, _http_version, _headers, body) = req.deconstruct();
        println!("{} {}", method, uri);

        self.root.lookup(uri.path(), uri.query(), None /*uri.fragment()*/)
            .and_then(move |resource| match resource {
                Some(resource) => {
                    use hyper::Method::*;
                    match method {
                        Options => futures::finished(resource.options()).boxed(),
                        Head => resource.head(),
                        Get => resource.get(),
                        Put => resource.put(body),
                        _ => futures::finished(resource.method_not_allowed()).boxed()
                    }
                },
                None => futures::finished(Self::not_found()).boxed()
            })
            .or_else(|err| Ok(Self::internal_server_error(err)))
            .boxed()
    }
}
