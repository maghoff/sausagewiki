use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use assets::StyleCss;
use mimes::*;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

pub struct ChangesResource {
    state: State,
    before: Option<i32>,
}

impl ChangesResource {
    pub fn new(state: State, before: Option<i32>) -> Self {
        Self { state, before }
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

            slug: String,
            title: String,

            _latest: bool,
        }

        #[derive(BartDisplay)]
        #[template="templates/changes.html"]
        struct Template<'a> {
            changes: &'a [Row],
        }

        let data = self.state.get_article_revision_stubs(self.before, 30);
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(data, head)| {
                use std::iter::Iterator;

                let changes = &data.into_iter().map(|x| {
                    Row {
                        _article_id: x.article_id,
                        _revision: x.revision,
                        created: Local.from_utc_datetime(&x.created).to_rfc2822(),
                        slug: x.slug,
                        title: x.title,
                        _latest: x.latest,
                    }
                }).collect::<Vec<_>>();

                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Changes",
                        body: &Template { changes },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }
}
