use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
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
            articles: &'a [ArticleReference],
        }

        struct ArticleReference {
            link: String,
            title: String,
        }

        let data = self.state.get_latest_article_revision_stubs();
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(articles, head)| {
                use std::iter::Iterator;

                let articles = &articles.into_iter().map(|x| {
                    ArticleReference {
                        link: if x.slug.is_empty() { ".".to_owned() } else { x.slug },
                        title: x.title,
                    }
                }).collect::<Vec<_>>();

                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Sitemap",
                        body: &Template { articles },
                    }.to_string()))
            }))
    }
}
