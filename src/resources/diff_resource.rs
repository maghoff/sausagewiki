use std::fmt;

use futures::future::done;
use futures::{self, Future};

use hyper::header::ContentType;
use hyper::server::*;

use crate::mimes::*;
use crate::models::ArticleRevision;
use crate::site::Layout;
use crate::state::State;
use crate::theme;
use crate::web::{Resource, ResponseFuture};

use super::changes_resource;
use super::pagination::Pagination;

type BoxResource = Box<dyn Resource + Sync + Send>;

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

    pub fn lookup(
        &self,
        article_id: u32,
        query: Option<&str>,
    ) -> Box<dyn Future<Item = Option<BoxResource>, Error = crate::web::Error>> {
        let state = self.state.clone();

        Box::new(
            done(serde_urlencoded::from_str(query.unwrap_or("")).map_err(Into::into))
                .and_then(move |params: QueryParameters| {
                    let from = state.get_article_revision(article_id as i32, params.from as i32);
                    let to = state.get_article_revision(article_id as i32, params.to as i32);

                    from.join(to)
                })
                .and_then(move |(from, to)| match (from, to) {
                    (Some(from), Some(to)) => {
                        Ok(Some(Box::new(DiffResource::new(from, to)) as BoxResource))
                    }
                    _ => Ok(None),
                }),
        )
    }
}

pub struct DiffResource {
    from: ArticleRevision,
    to: ArticleRevision,
}

impl DiffResource {
    pub fn new(from: ArticleRevision, to: ArticleRevision) -> Self {
        assert_eq!(from.article_id, to.article_id);
        Self { from, to }
    }
}

impl Resource for DiffResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(
            Response::new()
                .with_status(hyper::StatusCode::Ok)
                .with_header(ContentType(TEXT_HTML.clone())),
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template = "templates/diff.html"]
        struct Template<'a> {
            consecutive: bool,
            article_id: u32,
            author: Option<&'a str>,
            author_link: &'a str,
            article_history_link: &'a str,
            from_link: &'a str,
            to_link: &'a str,
            title: &'a [Diff<char>],
            lines: &'a [Diff<&'a str>],
        }

        #[derive(Default)]
        struct Diff<T: fmt::Display> {
            removed: Option<T>,
            same: Option<T>,
            added: Option<T>,
        }

        let head = self.head();

        Box::new(head.and_then(move |head| {
            let consecutive = self.to.revision - self.from.revision == 1;

            let author = match consecutive {
                true => self.to.author.as_deref(),
                false => None,
            };

            let author_link = &format!(
                "_changes{}",
                changes_resource::QueryParameters::default()
                    .author(author.map(|x| x.to_owned()))
                    .pagination(Pagination::After(self.from.sequence_number))
                    .into_link()
            );

            let article_history_link = &format!(
                "_changes{}",
                changes_resource::QueryParameters::default()
                    .article_id(Some(self.from.article_id))
                    .pagination(Pagination::After(self.from.sequence_number))
                    .into_link()
            );

            let title = &diff::chars(&self.from.title, &self.to.title)
                .into_iter()
                .map(|x| match x {
                    diff::Result::Left(x) => Diff {
                        removed: Some(x),
                        ..Default::default()
                    },
                    diff::Result::Both(x, _) => Diff {
                        same: Some(x),
                        ..Default::default()
                    },
                    diff::Result::Right(x) => Diff {
                        added: Some(x),
                        ..Default::default()
                    },
                })
                .collect::<Vec<_>>();

            let lines = &diff::lines(&self.from.body, &self.to.body)
                .into_iter()
                .map(|x| match x {
                    diff::Result::Left(x) => Diff {
                        removed: Some(x),
                        ..Default::default()
                    },
                    diff::Result::Both(x, _) => Diff {
                        same: Some(x),
                        ..Default::default()
                    },
                    diff::Result::Right(x) => Diff {
                        added: Some(x),
                        ..Default::default()
                    },
                })
                .collect::<Vec<_>>();

            Ok(head.with_body(
                Layout {
                    base: Some("../"), // Hmm, should perhaps accept `base` as argument
                    title: "Difference",
                    theme: theme::theme_from_str_hash("Difference"),
                    body: &Template {
                        consecutive,
                        article_id: self.from.article_id as u32,
                        author,
                        author_link,
                        article_history_link,
                        from_link: &format!(
                            "_revisions/{}/{}",
                            self.from.article_id, self.from.revision
                        ),
                        to_link: &format!("_revisions/{}/{}", self.to.article_id, self.to.revision),
                        title,
                        lines,
                    },
                }
                .to_string(),
            ))
        }))
    }
}
