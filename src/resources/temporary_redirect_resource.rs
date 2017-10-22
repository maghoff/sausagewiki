use futures::{self, Future};
use hyper;
use hyper::header::Location;
use hyper::server::*;

use web::{Resource, ResponseFuture};

pub struct TemporaryRedirectResource {
    location: String,
}

impl TemporaryRedirectResource {
    pub fn new(location: String) -> Self {
        Self { location }
    }
}

impl Resource for TemporaryRedirectResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put, Post]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::TemporaryRedirect)
            .with_header(Location::new(self.location.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(format!("Moved to {}", self.location)))
            }))
    }

    fn put(self: Box<Self>, _body: hyper::Body, _identity: Option<String>) -> ResponseFuture {
        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(format!("Moved to {}", self.location)))
            }))
    }
}
