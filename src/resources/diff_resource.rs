use std::{self, fmt};

use diff;
use futures::{self, Future};
use futures::future::done;
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_urlencoded;

use mimes::*;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

type BoxResource = Box<Resource + Sync + Send>;
type Error = Box<std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct DiffLookup {
    state: State,
}

#[derive(Serialize, Deserialize)]
pub struct QueryParameters {
    from: u32,
    to: u32,
}

impl QueryParameters {
    pub fn new(from: u32, to: u32) -> QueryParameters {
        QueryParameters { from, to }
    }
}

impl fmt::Display for QueryParameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&serde_urlencoded::to_string(self).expect("Serializing to String cannot fail"))
    }
}

impl DiffLookup {
    pub fn new(state: State) -> DiffLookup {
        Self { state }
    }

    pub fn lookup(&self, article_id: u32, query: Option<&str>) -> Box<Future<Item=Option<BoxResource>, Error=::web::Error>> {
        let state = self.state.clone();

        Box::new(done((|| -> Result<Option<BoxResource>, ::web::Error> {
            let params: QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

            Ok(Some(Box::new(DiffResource::new(state, article_id, params.from, params.to))))
        }())))
    }
}

pub struct DiffResource {
    state: State,
    article_id: u32,
    from: u32,
    to: u32,
}

impl DiffResource {
    pub fn new(state: State, article_id: u32, from: u32, to: u32) -> Self {
        Self { state, article_id, from, to }
    }

    fn query_args(&self) -> QueryParameters {
        QueryParameters {
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }
}

impl Resource for DiffResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template = "templates/diff.html"]
        struct Template<'a> {
            title: &'a [Diff<char>],
            lines: &'a [Diff<&'a str>],
        }

        #[derive(Default)]
        struct Diff<T: fmt::Display> {
            removed: Option<T>,
            same: Option<T>,
            added: Option<T>,
        }

        let from = self.state.get_article_revision(self.article_id as i32, self.from as i32);
        let to = self.state.get_article_revision(self.article_id as i32, self.to as i32);

        let head = self.head();

        Box::new(head.join3(from, to)
            .and_then(move |(head, from, to)| {
                Ok(head
                    .with_body(Layout {
                        base: Some("../"), // Hmm, should perhaps accept `base` as argument
                        title: "Difference",
                        body: &Template {
                            title: &diff::chars(
                                from.as_ref().map(|x| &*x.title).unwrap_or(""),
                                to.as_ref().map(|x| &*x.title).unwrap_or("")
                            )
                                .into_iter()
                                .map(|x| match x {
                                    diff::Result::Left(x) => Diff { removed: Some(x), ..Default::default() },
                                    diff::Result::Both(x, _) => Diff { same: Some(x), ..Default::default() },
                                    diff::Result::Right(x) => Diff { added: Some(x), ..Default::default() },
                                })
                                .collect::<Vec<_>>(),
                            lines: &diff::lines(
                                from.as_ref().map(|x| &*x.body).unwrap_or(""),
                                to.as_ref().map(|x| &*x.body).unwrap_or("")
                            )
                                .into_iter()
                                .map(|x| match x {
                                    diff::Result::Left(x) => Diff { removed: Some(x), ..Default::default() },
                                    diff::Result::Both(x, _) => Diff { same: Some(x), ..Default::default() },
                                    diff::Result::Right(x) => Diff { added: Some(x), ..Default::default() },
                                })
                                .collect::<Vec<_>>()
                        },
                    }.to_string()))
            }))
    }
}
