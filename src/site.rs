extern crate futures;

use hyper;
use hyper::server::*;

pub struct Site {
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, Self::Error>;

    fn call(&self, _req: Request) -> Self::Future {
        unimplemented!()
    }
}
