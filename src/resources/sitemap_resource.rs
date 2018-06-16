use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
use models::ArticleRevisionStub;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

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
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template="templates/sitemap.html"]
        struct Template<'a> {
            articles: &'a [ArticleRevisionStub],
        }

        let data = self.state.get_latest_article_revision_stubs();
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(articles, head)| {
                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Sitemap",
                        body: &Template {
                            articles: &articles,
                        },
                    }.to_string()))
            }))
    }
}
