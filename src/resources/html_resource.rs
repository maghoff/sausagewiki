use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
use site::Layout;
use web::{Resource, ResponseFuture};

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

#[derive(BartDisplay)]
#[template="templates/simple.html"]
struct Template<'a> {
    title: &'a str,
    html_body: &'a str,
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
                Ok(head
                    .with_body(Layout {
                        base: self.base,
                        title: self.title,
                        body: &Template {
                            title: self.title,
                            html_body: self.html_body,
                        },
                    }.to_string()))
            }))
    }
}
