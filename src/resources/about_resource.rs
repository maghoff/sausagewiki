use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
use site::Layout;
use web::{Resource, ResponseFuture};

pub struct AboutResource;

impl AboutResource {
    pub fn new() -> Self {
        AboutResource
    }
}

#[derive(BartDisplay)]
#[template="templates/about.html"]
struct Template;

impl Template {
    fn pkg_version(&self) -> &str { env!("CARGO_PKG_VERSION") }
}

impl Resource for AboutResource {
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
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "About Sausagewiki",
                        body: &Template,
                    }.to_string()))
            }))
    }
}
