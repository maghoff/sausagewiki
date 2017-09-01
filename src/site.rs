use std::fmt;

use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;

use models;
use state::State;
use web::{Lookup, Resource};

fn render_markdown(src: &str) -> String {
    use pulldown_cmark::{Parser, html};

    let p = Parser::new(src);
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

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

struct WikiLookup {
    state: State,
}

impl Lookup for WikiLookup {
    type Resource = ArticleResource;
    type Error = Box<::std::error::Error + Send>;
    type Future = futures::future::FutureResult<Option<Self::Resource>, Self::Error>;

    fn lookup(&self, path: &str, _query: Option<&str>, _fragment: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));

        if path.starts_with("/_") {
            // Reserved namespace
            return futures::finished(None);
        }

        let slug = &path[1..];
        if let Ok(article_id) = slug.parse() {
            match self.state.get_article_revision_by_id(article_id) {
                Ok(Some(article)) => {
                    futures::finished(Some(ArticleResource::new(article)))
                },
                Ok(None) => futures::finished(None),
                Err(err) => futures::failed(err),
            }
        } else {
            futures::finished(None)
        }
    }
}

struct ArticleResource {
    data: models::ArticleRevision,
}

impl ArticleResource {
    fn new(data: models::ArticleRevision) -> Self {
        Self {
            data
        }
    }
}

impl Resource for ArticleResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> futures::BoxFuture<Response, Box<::std::error::Error + Send>> {
        futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ).boxed()
    }

    fn get(self) -> futures::BoxFuture<Response, Box<::std::error::Error + Send>> {
        use chrono::{self, TimeZone, Local};

        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: i32,
            revision: i32,
            created: &'a chrono::DateTime<Local>,

            title: &'a str,
            body: String,
        }

        self.head().map(move |head|
            head
                .with_body(Layout {
                    title: &self.data.title,
                    body: &Template {
                        article_id: self.data.article_id,
                        revision: self.data.revision,
                        created: &Local.from_utc_datetime(&self.data.created),
                        title: &self.data.title,
                        body: render_markdown(&self.data.body),
                    }
                }.to_string())
        ).boxed()
    }
}


pub struct Site {
    root: WikiLookup,
}

impl Site {
    pub fn new(state: State) -> Site {
        Site {
            root: WikiLookup { state }
        }
    }

    fn not_found() -> Response {
        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Not found",
                body: &NotFound,
            }.to_string())
            .with_status(hyper::StatusCode::NotFound)
    }

    fn internal_server_error(err: Box<::std::error::Error + Send>) -> Response {
        eprintln!("Internal Server Error:\n{:#?}", err);

        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Internal server error",
                body: &InternalServerError,
            }.to_string())
            .with_status(hyper::StatusCode::InternalServerError)
    }
}

impl Service for Site {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let (method, uri, _http_version, _headers, _body) = req.deconstruct();
        println!("{} {}", method, uri);

        self.root.lookup(uri.path(), uri.query(), None /*uri.fragment()*/)
            .and_then(move |resource| match resource {
                Some(resource) => {
                    use hyper::Method::*;
                    match method {
                        Options => futures::finished(resource.options()).boxed(),
                        Head => resource.head(),
                        Get => resource.get(),
                        _ => futures::finished(resource.method_not_allowed()).boxed()
                    }
                },
                None => futures::finished(Self::not_found()).boxed()
            })
            .or_else(|err| Ok(Self::internal_server_error(err)))
            .boxed()
    }
}
