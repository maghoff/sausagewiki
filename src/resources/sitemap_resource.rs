use futures::{self, Future};

use hyper::header::ContentType;
use hyper::server::*;

use crate::mimes::*;
use crate::models::ArticleRevisionStub;
use crate::site::system_page;
use crate::state::State;
use crate::web::{Resource, ResponseFuture};

pub struct SitemapResource {
    state: State,
}

impl SitemapResource {
    pub fn new(state: State) -> Self {
        SitemapResource { state }
    }
}

impl Resource for SitemapResource {
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
        #[template = "templates/sitemap.html"]
        struct Template<'a> {
            articles: &'a [ArticleRevisionStub],
        }

        let data = self.state.get_latest_article_revision_stubs();
        let head = self.head();

        Box::new(data.join(head).and_then(move |(articles, head)| {
            Ok(head.with_body(
                system_page(
                    None, // Hmm, should perhaps accept `base` as argument
                    "Sitemap",
                    Template {
                        articles: &articles,
                    },
                )
                .to_string(),
            ))
        }))
    }
}
