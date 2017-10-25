use futures::{self, Future};
use hyper;
use hyper::header::{Accept, ContentType};
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use mimes::*;
use models;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

const DEFAULT_LIMIT: i32 = 10;
const DEFAULT_SNIPPET_SIZE: i32 = 8;

type BoxResource = Box<Resource + Sync + Send>;

#[derive(Serialize, Deserialize, Default)]
pub struct QueryParameters {
    q: Option<String>,
    offset: Option<i32>,
    limit: Option<i32>,
    snippet_size: Option<i32>,
}

impl QueryParameters {
    pub fn offset(self, offset: i32) -> Self {
        Self {
            offset: if offset != 0 { Some(offset) } else { None },
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

        Ok(Some(Box::new(
            SearchResource::new(
                self.state.clone(),
                args.q,
                args.limit.unwrap_or(DEFAULT_LIMIT),
                args.offset.unwrap_or(0),
                args.snippet_size.unwrap_or(DEFAULT_SNIPPET_SIZE),
            )
        )))
    }
}

pub struct SearchResource {
    state: State,
    response_type: ResponseType,

    query: Option<String>,
    limit: i32,
    offset: i32,
    snippet_size: i32,
}

// This is a complete hack, searching for a reasonable design:
pub enum ResponseType {
    Html,
    Json,
}

impl SearchResource {
    pub fn new(state: State, query: Option<String>, limit: i32, offset: i32, snippet_size: i32) -> Self {
        Self { state, response_type: ResponseType::Html, query, limit, offset, snippet_size }
    }
}

impl Resource for SearchResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    // This is a complete hack, searching for a reasonable design:
    fn hacky_inject_accept_header(&mut self, accept: Accept) {
        use hyper::header::QualityItem;
        use hyper::mime;

        self.response_type = match accept.first() {
            Some(&QualityItem { item: ref mime, .. })
                if mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON
                => ResponseType::Json,
            _ => ResponseType::Html,
        };
    }

    fn head(&self) -> ResponseFuture {
        let content_type = match &self.response_type {
            &ResponseType::Json => ContentType(APPLICATION_JSON.clone()),
            &ResponseType::Html => ContentType(TEXT_HTML.clone()),
        };

        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(content_type)
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template="templates/search.html"]
        struct Template<'a> {
            query: &'a str,
            hits: Vec<models::SearchResult>,
        }

        #[derive(Serialize)]
        struct JsonResponse<'a> {
            query: &'a str,
            hits: &'a [models::SearchResult],
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
        let query = self.query.as_ref().map(|x| x.clone()).unwrap_or("".to_owned());

        let data = self.state.search_query(query, self.limit, self.offset, self.snippet_size);
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(data, head)| {
                match &self.response_type {
                    &ResponseType::Json => Ok(head
                        .with_body(serde_json::to_string(&JsonResponse {
                            query: self.query.as_ref().map(|x| &**x).unwrap_or(""),
                            hits: &data,
                        }).expect("Should never fail"))
                    ),
                    &ResponseType::Html => Ok(head
                        .with_body(Layout {
                            base: None, // Hmm, should perhaps accept `base` as argument
                            title: "Search",
                            body: &Template {
                                query: self.query.as_ref().map(|x| &**x).unwrap_or(""),
                                hits: data,
                            },
                        }.to_string())),
                }
            }))
    }
}
