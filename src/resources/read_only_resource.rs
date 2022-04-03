use futures::Future;
use hyper::header::{ContentType, ContentLength, CacheControl, CacheDirective};
use hyper::server::*;
use hyper::StatusCode;

use crate::web::{Resource, ResponseFuture};

#[allow(unused)]
pub struct ReadOnlyResource {
    pub content_type: ::hyper::mime::Mime,
    pub body: Vec<u8>,
}

impl Resource for ReadOnlyResource {
    fn allow(&self) -> Vec<::hyper::Method> {
        use ::hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(::futures::finished(Response::new()
            .with_status(StatusCode::Ok)
            .with_header(ContentType(self.content_type.clone()))
            .with_header(CacheControl(vec![
                CacheDirective::MustRevalidate,
                CacheDirective::NoStore,
            ]))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        Box::new(self.head().map(move |head|
            head
                .with_header(ContentLength(self.body.len() as u64))
                .with_body(self.body.clone())
        ))
    }
}
