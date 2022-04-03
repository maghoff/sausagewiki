use futures::{self, Future};

use hyper::header::{Accept, ContentType};
use hyper::server::*;

use crate::mimes::*;
use crate::models::SearchResult;
use crate::site::system_page;
use crate::state::State;
use crate::web::{Resource, ResponseFuture};

const DEFAULT_LIMIT: u32 = 10;
const DEFAULT_SNIPPET_SIZE: u32 = 30;

type BoxResource = Box<dyn Resource + Sync + Send>;

#[derive(Serialize, Deserialize, Default)]
pub struct QueryParameters {
    q: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
    snippet_size: Option<u32>,
}

impl QueryParameters {
    pub fn offset(self, offset: u32) -> Self {
        Self {
            offset: if offset != 0 { Some(offset) } else { None },
            ..self
        }
    }

    pub fn limit(self, limit: u32) -> Self {
        Self {
            limit: if limit != DEFAULT_LIMIT {
                Some(limit)
            } else {
                None
            },
            ..self
        }
    }

    pub fn snippet_size(self, snippet_size: u32) -> Self {
        Self {
            snippet_size: if snippet_size != DEFAULT_SNIPPET_SIZE {
                Some(snippet_size)
            } else {
                None
            },
            ..self
        }
    }

    pub fn into_link(self) -> String {
        let args = serde_urlencoded::to_string(self).expect("Serializing to String cannot fail");
        if !args.is_empty() {
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

    pub fn lookup(&self, query: Option<&str>) -> Result<Option<BoxResource>, crate::web::Error> {
        let args: QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

        Ok(Some(Box::new(SearchResource::new(
            self.state.clone(),
            args.q,
            args.limit.unwrap_or(DEFAULT_LIMIT),
            args.offset.unwrap_or(0),
            args.snippet_size.unwrap_or(DEFAULT_SNIPPET_SIZE),
        ))))
    }
}

pub struct SearchResource {
    state: State,
    response_type: ResponseType,

    query: Option<String>,
    limit: u32,
    offset: u32,
    snippet_size: u32,
}

// This is a complete hack, searching for a reasonable design:
pub enum ResponseType {
    Html,
    Json,
}

impl SearchResource {
    pub fn new(
        state: State,
        query: Option<String>,
        limit: u32,
        offset: u32,
        snippet_size: u32,
    ) -> Self {
        Self {
            state,
            response_type: ResponseType::Html,
            query,
            limit,
            offset,
            snippet_size,
        }
    }

    fn query_args(&self) -> QueryParameters {
        QueryParameters {
            q: self.query.clone(),
            ..QueryParameters::default()
        }
        .offset(self.offset)
        .limit(self.limit)
        .snippet_size(self.snippet_size)
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
                if mime.type_() == mime::APPLICATION && mime.subtype() == mime::JSON =>
            {
                ResponseType::Json
            }
            _ => ResponseType::Html,
        };
    }

    fn head(&self) -> ResponseFuture {
        let content_type = match &self.response_type {
            &ResponseType::Json => ContentType(APPLICATION_JSON.clone()),
            &ResponseType::Html => ContentType(TEXT_HTML.clone()),
        };

        Box::new(futures::finished(
            Response::new()
                .with_status(hyper::StatusCode::Ok)
                .with_header(content_type),
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(Serialize)]
        struct JsonResponse<'a> {
            query: &'a str,
            hits: &'a [SearchResult],
            prev: Option<String>,
            next: Option<String>,
        }

        #[derive(BartDisplay)]
        #[template = "templates/search.html"]
        struct Template<'a> {
            query: &'a str,
            hits: &'a [(usize, &'a SearchResult)],
            prev: Option<String>,
            next: Option<String>,
        }

        // TODO: Show a search "front page" when no query is given:
        let query = self.query.as_ref().cloned().unwrap_or("".to_owned());

        let data = self.state.search_query(
            query,
            (self.limit + 1) as i32,
            self.offset as i32,
            self.snippet_size as i32,
        );
        let head = self.head();

        Box::new(data.join(head).and_then(move |(mut data, head)| {
            let prev = if self.offset > 0 {
                Some(
                    self.query_args()
                        .offset(self.offset.saturating_sub(self.limit))
                        .into_link(),
                )
            } else {
                None
            };

            let next = if data.len() > self.limit as usize {
                data.pop();
                Some(
                    self.query_args()
                        .offset(self.offset + self.limit)
                        .into_link(),
                )
            } else {
                None
            };

            match &self.response_type {
                &ResponseType::Json => Ok(head.with_body(
                    serde_json::to_string(&JsonResponse {
                        query: self.query.as_deref().unwrap_or(""),
                        hits: &data,
                        prev,
                        next,
                    })
                    .expect("Should never fail"),
                )),
                &ResponseType::Html => Ok(head.with_body(
                    system_page(
                        None, // Hmm, should perhaps accept `base` as argument
                        "Search",
                        &Template {
                            query: self.query.as_deref().unwrap_or(""),
                            hits: &data.iter().enumerate().collect::<Vec<_>>(),
                            prev,
                            next,
                        },
                    )
                    .to_string(),
                )),
            }
        }))
    }
}
