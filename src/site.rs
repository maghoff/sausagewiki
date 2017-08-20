use std::fmt;

use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;
use state::State;

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
}

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
struct Layout<'a, T: 'a + fmt::Display> {
    pub title: &'a str,
    pub body: &'a T,
}

#[derive(BartDisplay)]
#[template = "templates/404.html"]
struct NotFound;

#[derive(BartDisplay)]
#[template = "templates/500.html"]
struct InternalServerError;

pub struct Site {
    state: State
}

impl Site {
    pub fn new(state: State) -> Site {
        Site { state }
    }
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        println!("{} {}", req.method(), req.path());

        let path = req.path();

        if path.starts_with("/_") {
            futures::finished(
                Response::new()
                    .with_header(ContentType(TEXT_HTML.clone()))
                    .with_body(Layout {
                        title: "Not found",
                        body: &NotFound,
                    }.to_string())
                    .with_status(hyper::StatusCode::NotFound)
            ).boxed()
        } else {
            assert!(path.starts_with("/"));
            match self.state.find_article_by_slug(&path[1..]) {
                Ok(Some(article)) => {
                    futures::finished(
                        Response::new()
                            .with_header(ContentType(TEXT_HTML.clone()))
                            .with_body(Layout {
                                title: &article.title,
                                body: &article
                            }.to_string())
                            .with_status(hyper::StatusCode::Ok)
                    ).boxed()
                },
                Ok(None) => {
                    futures::finished(
                        Response::new()
                            .with_header(ContentType(TEXT_HTML.clone()))
                            .with_body(Layout {
                                title: "Not found",
                                body: &NotFound,
                            }.to_string())
                            .with_status(hyper::StatusCode::NotFound)
                    ).boxed()
                },
                Err(err) => {
                    eprintln!("Error while servicing request {} {}:\n{:#?}", req.method(), req.path(), err);
                    futures::finished(
                        Response::new()
                            .with_header(ContentType(TEXT_HTML.clone()))
                            .with_body(Layout {
                                title: "Internal server error",
                                body: &InternalServerError,
                            }.to_string())
                            .with_status(hyper::StatusCode::InternalServerError)
                    ).boxed()
                }
            }
        }
    }
}
