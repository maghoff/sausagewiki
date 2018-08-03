use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
use pagination::Pagination;
use resources::DiffQueryParameters;
use schema::article_revisions;
use site::system_page;
use state::State;
use web;

use super::apply_query_config;
use super::query_parameters;

pub struct Resource {
    state: State,
    show_authors: bool,
    before: Option<i32>,
    article_id: Option<i32>,
    author: Option<String>,
    limit: i32,
}

impl Resource {
    pub fn new(state: State, show_authors: bool, before: Option<i32>, article_id: Option<i32>, author: Option<String>, limit: i32) -> Self {
        Resource { state, show_authors, before, article_id, author, limit }
    }

    fn query_args(&self) -> query_parameters::QueryParameters {
        query_parameters::QueryParameters {
            after: None,
            before: self.before,
            article_id: self.article_id,
            author: self.author.clone(),
            ..query_parameters::QueryParameters::default()
        }
        .limit(self.limit)
    }
}

impl web::Resource for Resource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> web::ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> web::ResponseFuture {
        use chrono::{TimeZone, Local};

        struct Row<'a> {
            resource: &'a Resource,
            sequence_number: i32,

            article_id: i32,
            revision: i32,
            created: String,
            author: Option<String>,

            _slug: String,
            title: String,

            _latest: bool,

            diff_link: Option<String>,
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
            resource: &'a Resource,

            show_authors: bool,
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
                        diff_link:
                            if x.revision > 1 {
                                Some(format!("_diff/{}?{}",
                                    x.article_id,
                                    DiffQueryParameters::new(
                                        x.revision as u32 - 1,
                                        x.revision as u32,
                                    )
                                ))
                            } else {
                                None
                            },
                    }
                }).collect::<Vec<_>>();

                Ok(head.with_body(system_page(
                    None, // Hmm, should perhaps accept `base` as argument
                    "Changes",
                    Template {
                        resource: &self,
                        show_authors: self.show_authors,
                        newer,
                        older,
                        changes
                    }
                ).to_string()))
            }))
    }
}
