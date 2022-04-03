use futures::{Future, Stream};
use hyper::server::Response;
use hyper::{self, header, mime, server};

lazy_static! {
    static ref TEXT_PLAIN: mime::Mime = "text/plain;charset=utf-8".parse().unwrap();
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type ResponseFuture = Box<dyn futures::Future<Item = server::Response, Error = Error>>;

pub trait Resource {
    fn allow(&self) -> Vec<hyper::Method>;

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(self.method_not_allowed()))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        Box::new(futures::finished(self.method_not_allowed()))
    }

    fn put(self: Box<Self>, body: hyper::Body, _identity: Option<String>) -> ResponseFuture
    where
        Self: 'static,
    {
        Box::new(
            body.fold((), |_, _| -> Result<(), hyper::Error> { Ok(()) })
                .map_err(Into::into)
                .and_then(move |_| futures::finished(self.method_not_allowed())),
        )
    }

    fn post(self: Box<Self>, body: hyper::Body, _identity: Option<String>) -> ResponseFuture
    where
        Self: 'static,
    {
        Box::new(
            body.fold((), |_, _| -> Result<(), hyper::Error> { Ok(()) })
                .map_err(Into::into)
                .and_then(move |_| futures::finished(self.method_not_allowed())),
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

    fn hacky_inject_accept_header(&mut self, _: header::Accept) {
        // This function is a complete hack, searching for the appropriate
        // architecture.
    }
}
