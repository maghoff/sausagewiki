use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use crate::mimes::*;
use crate::site::system_page;
use crate::web::{Resource, ResponseFuture};

pub struct HtmlResource {
    base: Option<&'static str>,
    title: &'static str,
    html_body: &'static str,
}

impl HtmlResource {
    pub fn new(base: Option<&'static str>, title: &'static str, html_body: &'static str) -> Self {
        HtmlResource { base, title, html_body }
    }
}

impl Resource for HtmlResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        let head = self.head();

        Box::new(head
            .and_then(move |head| {
                Ok(head.with_body(system_page(
                    self.base,
                    self.title,
                    self.html_body
                ).to_string()))
            }))
    }
}
