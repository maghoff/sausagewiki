use diesel;
use futures::{self, Future};
use futures::future::{done, finished};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_urlencoded;

use assets::StyleCss;
use mimes::*;
use schema::article_revisions;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

use super::pagination::Pagination;
use super::TemporaryRedirectResource;

const DEFAULT_LIMIT: i32 = 30;

type BoxResource = Box<Resource + Sync + Send>;

#[derive(Clone)]
pub struct ChangesLookup {
    state: State,
}

// TODO: Optionally filter by article_id
// TODO: Optionally filter by author

#[derive(Serialize, Deserialize, Default)]
struct QueryParameters {
    after: Option<i32>,
    before: Option<i32>,

    limit: Option<i32>,
}

impl QueryParameters {
    fn pagination(self, pagination: Pagination<i32>) -> Self {
        Self {
            after: if let Pagination::After(x) = pagination { Some(x) } else { None },
            before: if let Pagination::Before(x) = pagination { Some(x) } else { None },
            ..self
        }
    }

    fn limit(self, limit: i32) -> Self {
        Self {
            limit: if limit != DEFAULT_LIMIT { Some(limit) } else { None },
            ..self
        }
    }

    fn into_link(self) -> String {
        let args = serde_urlencoded::to_string(self).expect("Serializing to String cannot fail");
        if args.len() > 0 {
            format!("?{}", args)
        } else {
            "_changes".to_owned()
        }
    }
}

fn apply_query_config<'a>(
    query: article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    limit: i32,
)
    -> article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>
{
    use diesel::prelude::*;

    query
        .limit(limit as i64 + 1)
}

impl ChangesLookup {
    pub fn new(state: State) -> ChangesLookup {
        Self { state }
    }

    pub fn lookup(&self, query: Option<&str>) -> Box<Future<Item=Option<BoxResource>, Error=::web::Error>> {
        use super::pagination;

        let state = self.state.clone();

        Box::new(
            done((|| {
                let params: QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

                let pagination = pagination::from_fields(params.after, params.before)?;

                let limit = match params.limit {
                    None => Ok(DEFAULT_LIMIT),
                    Some(x) if 1 <= x && x <= 100 => Ok(x),
                    _ => Err("`limit` argument must be in range [1, 100]"),
                }?;

                Ok((pagination, limit))
            })())
            .and_then(move |(pagination, limit)| match pagination {
                Pagination::After(x) => Box::new(
                    state.query_article_revision_stubs(move |query| {
                        use diesel::prelude::*;
                        use schema::article_revisions::dsl::*;

                        apply_query_config(query, limit)
                            .filter(sequence_number.gt(x))
                            .order(sequence_number.asc())
                    }).and_then(move |mut data| {
                        let extra_element = if data.len() > limit as usize {
                            data.pop()
                        } else {
                            None
                        };

                        Ok(Some(match extra_element {
                            Some(x) => Box::new(TemporaryRedirectResource::new(
                                QueryParameters::default()
                                    .limit(limit)
                                    .pagination(Pagination::Before(x.sequence_number))
                                    .into_link()
                            )) as BoxResource,
                            None => Box::new(TemporaryRedirectResource::new(
                                QueryParameters::default()
                                    .limit(limit)
                                    .into_link()
                            )) as BoxResource,
                        }))
                    })
                ) as Box<Future<Item=Option<BoxResource>, Error=::web::Error>>,
                Pagination::Before(x) => Box::new(finished(Some(Box::new(ChangesResource::new(state, Some(x), limit)) as BoxResource))),
                Pagination::None => Box::new(finished(Some(Box::new(ChangesResource::new(state, None, limit)) as BoxResource))),
            })
        )
    }
}

pub struct ChangesResource {
    state: State,
    before: Option<i32>,
    limit: i32,
}

impl ChangesResource {
    pub fn new(state: State, before: Option<i32>, limit: i32) -> Self {
        Self { state, before, limit }
    }

    fn query_args(&self) -> QueryParameters {
        QueryParameters {
            after: None,
            before: self.before,
            ..QueryParameters::default()
        }
        .limit(self.limit)
    }
}

impl Resource for ChangesResource {
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
        use chrono::{TimeZone, Local};

        struct Row {
            _article_id: i32,
            _revision: i32,
            created: String,
            author: Option<String>,

            slug: String,
            title: String,

            _latest: bool,
        }

        struct NavLinks {
            more: String,
            end: String,
        }

        #[derive(BartDisplay)]
        #[template="templates/changes.html"]
        struct Template<'a> {
            newer: Option<NavLinks>,
            older: Option<NavLinks>,
            changes: &'a [Row],
        }

        let before = self.before.clone();
        let limit = self.limit;
        let data = self.state.query_article_revision_stubs(move |query| {
            use diesel::prelude::*;
            use schema::article_revisions::dsl::*;

            let query = apply_query_config(query, limit)
                .order(sequence_number.desc());

            match before {
                Some(x) => query.filter(sequence_number.lt(x)),
                None => query,
            }
        });

        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(mut data, head)| {
                use std::iter::Iterator;

                if data.len() == 0 {
                    // TODO Handle degenerate case
                    unimplemented!("Cannot deal with empty result sets");
                }

                let extra_element = if data.len() > self.limit as usize {
                    data.pop()
                } else {
                    None
                };

                let (newer, older) = match self.before {
                    Some(x) => (
                        Some(NavLinks {
                            more: self.query_args().pagination(Pagination::After(x-1)).into_link(),
                            end: self.query_args().pagination(Pagination::None).into_link(),
                        }),
                        extra_element.map(|_| NavLinks {
                            more: self.query_args()
                                .pagination(Pagination::Before(data.last().unwrap().sequence_number))
                                .into_link(),
                            end: self.query_args().pagination(Pagination::After(0)).into_link(),
                        })
                    ),
                    None => (
                        None,
                        extra_element.map(|_| NavLinks {
                            more: self.query_args()
                                .pagination(Pagination::Before(data.last().unwrap().sequence_number))
                                .into_link(),
                            end: self.query_args().pagination(Pagination::After(0)).into_link(),
                        }),
                    ),
                };

                let changes = &data.into_iter().map(|x| {
                    Row {
                        _article_id: x.article_id,
                        _revision: x.revision,
                        created: Local.from_utc_datetime(&x.created).to_rfc2822(),
                        author: x.author,
                        slug: x.slug,
                        title: x.title,
                        _latest: x.latest,
                    }
                }).collect::<Vec<_>>();

                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Changes",
                        body: &Template {
                            newer,
                            older,
                            changes
                        },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }
}
