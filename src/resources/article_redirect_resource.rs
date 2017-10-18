use futures::{self, Future};
use hyper;
use hyper::header::Location;
use hyper::server::*;

use web::{Resource, ResponseFuture};

pub struct ArticleRedirectResource {
    slug: String,
}

impl ArticleRedirectResource {
    pub fn new(slug: String) -> Self {
        // Hack to let redirects to "" work:
        // TODO Calculate absolute Location URLs to conform to spec
        // This would also remove the need for this hack
        Self {
            slug: if slug == "" { ".".to_owned() } else { slug }
        }
    }
}

impl Resource for ArticleRedirectResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::TemporaryRedirect)
            .with_header(Location::new(self.slug.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(format!("Moved to {}", self.slug)))
            }))
    }

    fn put(self: Box<Self>, _body: hyper::Body, _identity: Option<String>) -> ResponseFuture {
        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(format!("Moved to {}", self.slug)))
            }))
    }
}
