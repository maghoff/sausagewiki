// #[derive(BartDisplay)] can cause unused extern crates warning:
#![allow(unused_extern_crates)]

use std::collections::HashMap;
use std::fmt;

use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use models;
use state::State;
use web::{Lookup, Resource};

use pulldown_cmark::Event;

struct EscapeHtml<'a, I: Iterator<Item=Event<'a>>> {
    inner: I,
}

impl<'a, I: Iterator<Item=Event<'a>>> EscapeHtml<'a, I> {
    fn new(inner: I) -> EscapeHtml<'a, I> {
        EscapeHtml { inner }
    }
}

impl<'a, I: Iterator<Item=Event<'a>>> Iterator for EscapeHtml<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use pulldown_cmark::Event::{Text, Html, InlineHtml};

        match self.inner.next() {
            Some(Html(x)) => Some(Text(x)),
            Some(InlineHtml(x)) => Some(Text(x)),
            x => x
        }
    }
}

fn render_markdown(src: &str) -> String {
    use pulldown_cmark::{Parser, html};

    let p = EscapeHtml::new(Parser::new(src));
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
    static ref APPLICATION_JSON: mime::Mime = "application/json".parse().unwrap();
}

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
struct Layout<'a, T: 'a + fmt::Display> {
    pub title: &'a str,
    pub body: &'a T,
    pub style_css_checksum: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/404.html"]
struct NotFound;

#[derive(BartDisplay)]
#[template = "templates/500.html"]
struct InternalServerError;

#[derive(StaticResource)]
#[filename = "assets/style.css"]
#[mime = "text/css"]
struct StyleCss;

#[derive(StaticResource)]
#[filename = "assets/script.js"]
#[mime = "application/javascript"]
struct ScriptJs;

// SIL Open Font License 1.1: http://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL
// Copyright 2015 The Amatic SC Project Authors (contact@sansoxygen.com)
#[derive(StaticResource)]
#[filename = "assets/amatic-sc-v9-latin-regular.woff"]
#[mime = "application/font-woff"]
struct AmaticFont;

struct WikiLookup {
    state: State,
    lookup_map: HashMap<String, Box<Fn() -> Box<Resource + Sync + Send>>>,
}

impl WikiLookup {
    fn new(state: State) -> WikiLookup {
        let mut lookup_map = HashMap::new();

        lookup_map.insert(
            format!("/_assets/style-{}.css", StyleCss::checksum()),
            Box::new(|| Box::new(StyleCss) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send>>
        );

        lookup_map.insert(
            format!("/_assets/script-{}.js", ScriptJs::checksum()),
            Box::new(|| Box::new(ScriptJs) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send>>
        );

        lookup_map.insert(
            format!("/_assets/amatic-sc-v9-latin-regular.woff"),
            Box::new(|| Box::new(AmaticFont) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send>>
        );

        WikiLookup { state, lookup_map }
    }
}

impl Lookup for WikiLookup {
    type Resource = Box<Resource + Send + Sync>;
    type Error = Box<::std::error::Error + Send + Sync>;
    type Future = futures::BoxFuture<Option<Self::Resource>, Self::Error>;

    fn lookup(&self, path: &str, _query: Option<&str>, _fragment: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));

        if path.starts_with("/_") {
            // Reserved namespace

            return futures::finished(
                self.lookup_map.get(path).map(|x| x())
            ).boxed();
        }

        let slug = &path[1..];
        if let Ok(article_id) = slug.parse() {
            let state = self.state.clone();
            self.state.get_article_revision_by_id(article_id)
                .and_then(|x| Ok(x.map(move |article| Box::new(ArticleResource::new(state, article)) as Box<Resource + Sync + Send>)))
                .boxed()
        } else {
            futures::finished(None).boxed()
        }
    }
}

struct ArticleResource {
    state: State,
    data: models::ArticleRevision,
}

impl ArticleResource {
    fn new(state: State, data: models::ArticleRevision) -> Self {
        Self { state, data }
    }
}

impl Resource for ArticleResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>> {
        futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ).boxed()
    }

    fn get(self: Box<Self>) -> futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>> {
        use chrono::{self, TimeZone, Local};

        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: i32,
            revision: i32,
            created: &'a chrono::DateTime<Local>,

            title: &'a str,
            raw: &'a str,
            rendered: String,

            script_js_checksum: &'a str,
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
                        raw: &self.data.body,
                        rendered: render_markdown(&self.data.body),
                        script_js_checksum: ScriptJs::checksum(),
                    },
                    style_css_checksum: StyleCss::checksum(),
                }.to_string())
        ).boxed()
    }

    fn put(self: Box<Self>, body: hyper::Body) -> futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>> {
        // TODO Check incoming Content-Type

        use chrono::{TimeZone, Local};
        use futures::Stream;

        #[derive(Deserialize)]
        struct UpdateArticle {
            base_revision: i32,
            body: String,
        }

        #[derive(Serialize)]
        struct PutResponse<'a> {
            revision: i32,
            rendered: &'a str,
            created: &'a str,
        }

        body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |update: UpdateArticle| {
                self.state.update_article(self.data.article_id, update.base_revision, update.body)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::Ok)
                    .with_header(ContentType(APPLICATION_JSON.clone()))
                    .with_body(serde_json::to_string(&PutResponse {
                        revision: updated.revision,
                        rendered: &render_markdown(&updated.body),
                        created: &Local.from_utc_datetime(&updated.created).to_string(),
                    }).expect("Should never fail"))
                )
            })
            .boxed()
    }
}


pub struct Site {
    root: WikiLookup,
}

impl Site {
    pub fn new(state: State) -> Site {
        Site {
            root: WikiLookup::new(state)
        }
    }

    fn not_found() -> Response {
        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Not found",
                body: &NotFound,
                style_css_checksum: StyleCss::checksum(),
            }.to_string())
            .with_status(hyper::StatusCode::NotFound)
    }

    fn internal_server_error(err: Box<::std::error::Error + Send + Sync>) -> Response {
        eprintln!("Internal Server Error:\n{:#?}", err);

        Response::new()
            .with_header(ContentType(TEXT_HTML.clone()))
            .with_body(Layout {
                title: "Internal server error",
                body: &InternalServerError,
                style_css_checksum: StyleCss::checksum(),
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
        let (method, uri, _http_version, _headers, body) = req.deconstruct();
        println!("{} {}", method, uri);

        self.root.lookup(uri.path(), uri.query(), None /*uri.fragment()*/)
            .and_then(move |resource| match resource {
                Some(resource) => {
                    use hyper::Method::*;
                    match method {
                        Options => futures::finished(resource.options()).boxed(),
                        Head => resource.head(),
                        Get => resource.get(),
                        Put => resource.put(body),
                        _ => futures::finished(resource.method_not_allowed()).boxed()
                    }
                },
                None => futures::finished(Self::not_found()).boxed()
            })
            .or_else(|err| Ok(Self::internal_server_error(err)))
            .boxed()
    }
}
