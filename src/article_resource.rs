use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use assets::{StyleCss, ScriptJs};
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
    static ref APPLICATION_JSON: mime::Mime = "application/json".parse().unwrap();
}

fn render_markdown(src: &str) -> String {
    use pulldown_cmark::Event;

    struct EscapeHtml<'a, I: Iterator<Item=Event<'a>>> {
        inner: I,
    }

    impl<'a, I: Iterator<Item=Event<'a>>> EscapeHtml<'a, I> {
        fn new(inner: I) -> EscapeHtml<'a, I> {
            EscapeHtml { inner }
        }
    }

    impl<'a, I: Iterator<Item=Event<'a>>> Iterator for EscapeHtml<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            use pulldown_cmark::Event::{Text, Html, InlineHtml};

            match self.inner.next() {
                Some(Html(x)) => Some(Text(x)),
                Some(InlineHtml(x)) => Some(Text(x)),
                x => x
            }
        }
    }

    use pulldown_cmark::{Parser, html};

    let p = EscapeHtml::new(Parser::new(src));
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

pub struct ArticleResource {
    state: State,
    article_id: i32,
    revision: i32,
}

impl ArticleResource {
    pub fn new(state: State, article_id: i32, revision: i32) -> Self {
        Self { state, article_id, revision }
    }
}

impl Resource for ArticleResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        use chrono::{self, TimeZone, Local};

        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: i32,
            revision: i32,
            created: &'a chrono::DateTime<Local>,

            title: &'a str,
            raw: &'a str,
            rendered: String,

            script_js_checksum: &'a str,
        }

        let data = self.state.get_article_revision(self.article_id, self.revision)
            .map(|x| x.expect("Data model guarantees that this exists"));
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(data, head)| {
                Ok(head
                    .with_body(Layout {
                        title: &data.title,
                        body: &Template {
                            article_id: data.article_id,
                            revision: data.revision,
                            created: &Local.from_utc_datetime(&data.created),
                            title: &data.title,
                            raw: &data.body,
                            rendered: render_markdown(&data.body),
                            script_js_checksum: ScriptJs::checksum(),
                        },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }

    fn put(self: Box<Self>, body: hyper::Body) -> ResponseFuture {
        // TODO Check incoming Content-Type

        use chrono::{TimeZone, Local};
        use futures::Stream;

        #[derive(Deserialize)]
        struct UpdateArticle {
            base_revision: i32,
            body: String,
        }

        #[derive(Serialize)]
        struct PutResponse<'a> {
            revision: i32,
            rendered: &'a str,
            created: &'a str,
        }

        Box::new(body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |update: UpdateArticle| {
                self.state.update_article(self.article_id, update.base_revision, update.body)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::Ok)
                    .with_header(ContentType(APPLICATION_JSON.clone()))
                    .with_body(serde_json::to_string(&PutResponse {
                        revision: updated.revision,
                        rendered: &render_markdown(&updated.body),
                        created: &Local.from_utc_datetime(&updated.created).to_string(),
                    }).expect("Should never fail"))
                )
            })
        )
    }
}
