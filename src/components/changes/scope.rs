use futures::Future;
use futures::future::{done, finished};
use serde_urlencoded;

use pagination::{self, Pagination};
use resources::TemporaryRedirectResource;
use schema::article_revisions;
use state::State;
use web;

use super::apply_query_config;
use super::query_parameters;
use super::Resource;

type BoxResource = Box<web::Resource + Sync + Send>;

#[derive(Clone)]
pub struct Scope {
    state: State,
    show_authors: bool,
}

impl Scope {
    pub fn new(state: State, show_authors: bool) -> Scope {
        Self { state, show_authors }
    }

    pub fn lookup(&self, query: Option<&str>) -> Box<Future<Item=Option<BoxResource>, Error=::web::Error>> {
        let state = self.state.clone();
        let show_authors = self.show_authors;

        Box::new(
            done((|| {
                let params: query_parameters::QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

                let pagination = pagination::from_fields(params.after, params.before)?;

                let limit = match params.limit {
                    None => Ok(query_parameters::DEFAULT_LIMIT),
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
                            query_parameters::QueryParameters {
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
                Pagination::Before(x) => Box::new(finished(Some(Box::new(Resource::new(state, show_authors, Some(x), article_id, author, limit)) as BoxResource))),
                Pagination::None => Box::new(finished(Some(Box::new(Resource::new(state, show_authors, None, article_id, author, limit)) as BoxResource))),
            })
        )
    }
}
