use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_urlencoded;

use assets::StyleCss;
use mimes::*;
use models;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

const DEFAULT_LIMIT: i32 = 30;

type BoxResource = Box<Resource + Sync + Send>;

#[derive(Serialize, Deserialize, Default)]
pub struct QueryParameters {
    q: Option<String>,
    skip: Option<i32>,
    limit: Option<i32>,
}

impl QueryParameters {
    pub fn skip(self, skip: i32) -> Self {
        Self {
            skip: if skip != 0 { Some(skip) } else { None },
            ..self
        }
    }

    pub fn limit(self, limit: i32) -> Self {
        Self {
            limit: if limit != DEFAULT_LIMIT { Some(limit) } else { None },
            ..self
        }
    }

    pub fn into_link(self) -> String {
        let args = serde_urlencoded::to_string(self).expect("Serializing to String cannot fail");
        if args.len() > 0 {
            format!("_search?{}", args)
        } else {
            "_search".to_owned()
        }
    }
}

#[derive(Clone)]
pub struct SearchLookup {
    state: State,
}

impl SearchLookup {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn lookup(&self, query: Option<&str>) -> Result<Option<BoxResource>, ::web::Error> {
        let args: QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

        Ok(Some(Box::new(SearchResource::new(self.state.clone(), args))))
    }
}

pub struct SearchResource {
    state: State,
    query_args: QueryParameters,
}

impl SearchResource {
    pub fn new(state: State, query_args: QueryParameters) -> Self {
        Self { state, query_args }
    }
}

impl Resource for SearchResource {
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
        #[template="templates/search.html"]
        struct Template<'a> {
            query: &'a str,
            hits: Vec<models::SearchResult>,
        }

        impl models::SearchResult {
            fn link(&self) -> String {
                if self.slug == "" {
                    ".".to_owned()
                } else {
                    self.slug.clone()
                }
            }
        }

        // TODO: Show a search "front page" when no query is given:
        let query = self.query_args.q.as_ref().map(|x| x.clone()).unwrap_or("".to_owned());

        let data = self.state.search_query(query);
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(data, head)| {
                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Search",
                        body: &Template {
                            query: self.query_args.q.as_ref().map(|x| &**x).unwrap_or(""),
                            hits: data,
                        },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }
}
