use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
}

pub struct Site {
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, Self::Error>;

    fn call(&self, _req: Request) -> Self::Future {
        futures::finished(
            Response::new()
                .with_header(ContentType(TEXT_HTML.clone()))
                .with_body(format!("WOOOOOOOOOOT!"))
                .with_status(hyper::StatusCode::Ok)
        ).boxed()
    }
}
