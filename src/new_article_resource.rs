use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use assets::{StyleCss, ScriptJs};
use mimes::*;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

const NDASH: &str = "\u{2013}";

const EMPTY_ARTICLE_MESSAGE: &str = "
<p>Not found</p>
<p>There's no article here yet. You can create one by clicking the
edit-link below and saving a new article.</p>
";

fn title_from_slug(slug: &str) -> String {
    slug.replace('-', " ")
}

pub struct NewArticleResource {
    state: State,
    slug: String,
}

impl NewArticleResource {
    pub fn new(state: State, slug: String) -> Self {
        Self { state, slug }
    }
}

impl Resource for NewArticleResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::NotFound)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: &'a str,
            revision: &'a str,
            created: &'a str,

            slug: &'a str,
            title: &'a str,
            raw: &'a str,
            rendered: &'a str,

            script_js_checksum: &'a str,
        }

        let title = title_from_slug(&self.slug);

        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(Layout {
                        title: &title,
                        body: &Template {
                            article_id: NDASH,
                            revision: NDASH,
                            created: NDASH,
                            slug: &self.slug,
                            title: &title,
                            raw: "",
                            rendered: EMPTY_ARTICLE_MESSAGE,
                            script_js_checksum: ScriptJs::checksum(),
                        },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }

    fn put(self: Box<Self>, _body: hyper::Body) -> ResponseFuture {
        unimplemented!()
    }
}
