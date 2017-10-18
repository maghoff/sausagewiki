use futures;
use hyper::{self, header, mime, server};
use hyper::server::Response;
use std;

lazy_static! {
    static ref TEXT_PLAIN: mime::Mime = "text/plain;charset=utf-8".parse().unwrap();
}

pub type Error = Box<std::error::Error + Send + Sync>;
pub type ResponseFuture = Box<futures::Future<Item = server::Response, Error = Error>>;

pub trait Resource {
    fn allow(&self) -> Vec<hyper::Method>;

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(self.method_not_allowed()))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        Box::new(futures::finished(self.method_not_allowed()))
    }

    fn put(self: Box<Self>, body: hyper::Body, _identity: Option<String>) -> ResponseFuture
        where Self: 'static
    {
        use futures::{Future, Stream};

        // TODO Cleanup by moving to the built in never type, !, when it stabilizes
        enum Never {};
        impl std::convert::From<Never> for hyper::Error {
            fn from(_: Never) -> hyper::Error {
                panic!()
            }
        }

        Box::new(body
            .fold((), |_, _| -> Result<(), Never> { Ok(()) })
            .map_err(Into::into)
            .and_then(move |_| futures::finished(self.method_not_allowed()))
        )
    }

    fn options(&self) -> Response {
        Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(header::Allow(self.allow()))
    }

    fn method_not_allowed(&self) -> Response {
        Response::new()
            .with_status(hyper::StatusCode::MethodNotAllowed)
            .with_header(header::Allow(self.allow()))
            .with_header(header::ContentType(TEXT_PLAIN.clone()))
            .with_body("Method not allowed\n")
    }
}
