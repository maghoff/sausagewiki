use futures::{self, Future};
use futures::future::finished;
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use assets::StyleCss;
use mimes::*;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

use super::pagination::Pagination;

const PAGE_SIZE: i32 = 30;

pub struct ChangesResource {
    state: State,
    pagination: Pagination<i32>,
}

impl ChangesResource {
    pub fn new(state: State, pagination: Pagination<i32>) -> Box<Future<Item=ChangesResource, Error=::web::Error>> {
        match pagination {
            Pagination::After(x) => Box::new(
                state.query_article_revision_stubs(move |query| {
                    use diesel::prelude::*;
                    use schema::article_revisions::dsl::*;

                    query
                        .limit(PAGE_SIZE as i64 + 1)
                        .filter(sequence_number.gt(x))
                        .order(sequence_number.asc())
                }).and_then(|mut data| {
                    let extra_element = if data.len() > PAGE_SIZE as usize {
                        data.pop()
                    } else {
                        None
                    };

                    Ok(Self {
                        state,
                        pagination: match extra_element {
                            Some(x) => Pagination::Before(x.sequence_number),
                            None => Pagination::None
                        }
                    })
                })
            ),
            Pagination::Before(x) => Box::new(finished(Self { state, pagination })),
            Pagination::None => Box::new(finished(Self { state, pagination })),
        }
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

        let pagination = self.pagination.clone();
        let data = self.state.query_article_revision_stubs(move |query| {
            use diesel::prelude::*;
            use schema::article_revisions::dsl::*;

            let query = query
                .limit(PAGE_SIZE as i64 + 1);

            match pagination {
                Pagination::After(x) => query
                    .filter(sequence_number.gt(x))
                    .order(sequence_number.asc()),
                Pagination::Before(x) => query
                    .filter(sequence_number.lt(x))
                    .order(sequence_number.desc()),
                Pagination::None => query
                    .order(sequence_number.desc()),
            }
        });

        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(mut data, head)| {
                use std::iter::Iterator;

                let extra_element = if data.len() > PAGE_SIZE as usize {
                    data.pop()
                } else {
                    None
                };

                let (newer, older) = match self.pagination {
                    Pagination::After(x) => {
                        data.reverse();
                        (
                            extra_element.map(|_| NavLinks {
                                more: format!("?after={}", data.first().unwrap().sequence_number),
                                end: format!("_changes"),
                            }),
                            Some(NavLinks {
                                more: format!("?before={}", x + 1),
                                end: format!("?after=0"),
                            })
                        )
                    },
                    Pagination::Before(x) => (
                        Some(NavLinks {
                            more: format!("?after={}", x - 1),
                            end: format!("_changes"),
                        }),
                        extra_element.map(|_| NavLinks {
                            more: format!("?before={}", data.last().unwrap().sequence_number),
                            end: format!("?after=0"),
                        })
                    ),
                    Pagination::None => (
                        None,
                        extra_element.map(|_| NavLinks {
                            more: format!("?before={}", data.last().unwrap().sequence_number),
                            end: format!("?after=0"),
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
