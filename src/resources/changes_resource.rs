use diesel;
use futures::{self, Future};
use futures::future::{done, finished};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_urlencoded;

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

#[derive(Serialize, Deserialize, Default)]
pub struct QueryParameters {
    after: Option<i32>,
    before: Option<i32>,

    article_id: Option<i32>,
    author: Option<String>,

    limit: Option<i32>,
}

impl QueryParameters {
    pub fn pagination(self, pagination: Pagination<i32>) -> Self {
        Self {
            after: if let Pagination::After(x) = pagination { Some(x) } else { None },
            before: if let Pagination::Before(x) = pagination { Some(x) } else { None },
            ..self
        }
    }

    pub fn article_id(self, article_id: Option<i32>) -> Self {
        Self { article_id, ..self }
    }

    pub fn author(self, author: Option<String>) -> Self {
        Self { author, ..self }
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
            format!("?{}", args)
        } else {
            "_changes".to_owned()
        }
    }
}

fn apply_query_config<'a>(
    mut query: article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    article_id: Option<i32>,
    author: Option<String>,
    limit: i32,
)
    -> article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>
{
    use diesel::prelude::*;

    if let Some(article_id) = article_id {
        query = query.filter(article_revisions::article_id.eq(article_id));
    }

    if let Some(author) = author {
        query = query.filter(article_revisions::author.eq(author));
    }

    query.limit(limit as i64 + 1)
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

                Ok((pagination, params.article_id, params.author, limit))
            })())
            .and_then(move |(pagination, article_id, author, limit)| match pagination {
                Pagination::After(x) => {
                    let author2 = author.clone();

                    Box::new(state.query_article_revision_stubs(move |query| {
                        use diesel::prelude::*;

                        apply_query_config(query, article_id, author2, limit)
                            .filter(article_revisions::sequence_number.gt(x))
                            .order(article_revisions::sequence_number.asc())
                    }).and_then(move |mut data| {
                        let extra_element = if data.len() > limit as usize {
                            data.pop()
                        } else {
                            None
                        };

                        let args =
                            QueryParameters {
                                after: None,
                                before: None,
                                article_id,
                                author,
                                limit: None,
                            }
                            .limit(limit);

                        Ok(Some(match extra_element {
                            Some(x) => Box::new(TemporaryRedirectResource::new(
                                args
                                    .pagination(Pagination::Before(x.sequence_number))
                                    .into_link()
                            )) as BoxResource,
                            None => Box::new(TemporaryRedirectResource::new(
                                args.into_link()
                            )) as BoxResource,
                        }))
                    })) as Box<Future<Item=Option<BoxResource>, Error=::web::Error>>
                },
                Pagination::Before(x) => Box::new(finished(Some(Box::new(ChangesResource::new(state, Some(x), article_id, author, limit)) as BoxResource))),
                Pagination::None => Box::new(finished(Some(Box::new(ChangesResource::new(state, None, article_id, author, limit)) as BoxResource))),
            })
        )
    }
}

pub struct ChangesResource {
    state: State,
    before: Option<i32>,
    article_id: Option<i32>,
    author: Option<String>,
    limit: i32,
}

impl ChangesResource {
    pub fn new(state: State, before: Option<i32>, article_id: Option<i32>, author: Option<String>, limit: i32) -> Self {
        Self { state, before, article_id, author, limit }
    }

    fn query_args(&self) -> QueryParameters {
        QueryParameters {
            after: None,
            before: self.before,
            article_id: self.article_id,
            author: self.author.clone(),
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

        struct Row<'a> {
            resource: &'a ChangesResource,
            sequence_number: i32,

            article_id: i32,
            revision: i32,
            created: String,
            author: Option<String>,

            _slug: String,
            title: String,

            _latest: bool,
        }

        impl<'a> Row<'a> {
            fn author_link(&self) -> String {
                self.resource.query_args()
                    .pagination(Pagination::After(self.sequence_number))
                    .author(self.author.clone())
                    .into_link()
            }
        }

        struct NavLinks {
            more: String,
            end: String,
        }

        #[derive(BartDisplay)]
        #[template="templates/changes.html"]
        struct Template<'a> {
            resource: &'a ChangesResource,

            newer: Option<NavLinks>,
            older: Option<NavLinks>,
            changes: &'a [Row<'a>],
        }

        impl<'a> Template<'a> {
            fn subject_clause(&self) -> String {
                match self.resource.article_id {
                    Some(x) => format!(" <a href=\"_by_id/{}\">this article</a>", x),
                    None => format!(" the wiki"),
                }
            }

            fn author(&self) -> Option<String> {
                self.resource.author.clone()
            }

            fn all_articles_link(&self) -> Option<String> {
                self.resource.article_id.map(|_| {
                    self.resource.query_args()
                        .article_id(None)
                        .into_link()
                })
            }

            fn all_authors_link(&self) -> Option<String> {
                self.resource.author.as_ref().map(|_| {
                    self.resource.query_args()
                        .author(None)
                        .into_link()
                })
            }
        }

        let (before, article_id, author, limit) =
            (self.before.clone(), self.article_id.clone(), self.author.clone(), self.limit);
        let data = self.state.query_article_revision_stubs(move |query| {
            use diesel::prelude::*;

            let query = apply_query_config(query, article_id, author, limit)
                .order(article_revisions::sequence_number.desc());

            match before {
                Some(x) => query.filter(article_revisions::sequence_number.lt(x)),
                None => query,
            }
        });

        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(mut data, head)| {
                use std::iter::Iterator;

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
                        resource: &self,
                        sequence_number: x.sequence_number,
                        article_id: x.article_id,
                        revision: x.revision,
                        created: Local.from_utc_datetime(&x.created).to_rfc2822(),
                        author: x.author,
                        _slug: x.slug,
                        title: x.title,
                        _latest: x.latest,
                    }
                }).collect::<Vec<_>>();

                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Changes",
                        body: &Template {
                            resource: &self,
                            newer,
                            older,
                            changes
                        },
                    }.to_string()))
            }))
    }
}
