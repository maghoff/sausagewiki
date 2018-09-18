use futures::{self, Future};
use hyper;
use hyper::header::{ContentType, Location};
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use assets::ScriptJs;
use mimes::*;
use rendering::render_markdown;
use site::Layout;
use state::State;
use theme;
use web::{Resource, ResponseFuture};

const NEW: &str = "NEW";

const EMPTY_ARTICLE_MESSAGE: &str = "
<p>Not found</p>
<p>There's no article here yet. You can create one by clicking the
edit-link below and saving a new article.</p>
";

fn title_from_slug(slug: &str) -> String {
    ::titlecase::titlecase(&slug.replace('-', " "))
}

pub struct NewArticleResource {
    state: State,
    slug: Option<String>,
}

#[derive(Deserialize)]
struct CreateArticle {
    base_revision: String,
    title: String,
    body: String,
}

impl NewArticleResource {
    pub fn new(state: State, slug: Option<String>) -> Self {
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
        #[template="templates/article.html"]
        struct Template<'a> {
            revision: &'a str,
            last_updated: Option<&'a str>,

            edit: bool,
            cancel_url: Option<&'a str>,
            title: &'a str,
            raw: &'a str,
            rendered: &'a str,
        }
        impl<'a> Template<'a> {
            fn script_js(&self) -> &'static str {
                ScriptJs::resource_name()
            }
        }

        let title = self.slug.as_ref()
            .map_or("".to_owned(), |x| title_from_slug(x));

        Box::new(self.head()
            .and_then(move |head| {
                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: &title,
                        theme: theme::theme_from_str(&title),
                        body: &Template {
                            revision: NEW,
                            last_updated: None,

                            // Implicitly start in edit-mode when no slug is given. This
                            // currently directly corresponds to the /_new endpoint
                            edit: self.slug.is_none(),

                            cancel_url: self.slug.as_ref().map(|x| &**x),
                            title: &title,
                            raw: "",
                            rendered: EMPTY_ARTICLE_MESSAGE,
                        },
                    }.to_string()))
            }))
    }

    fn put(self: Box<Self>, body: hyper::Body, identity: Option<String>) -> ResponseFuture {
        // TODO Check incoming Content-Type
        // TODO Refactor? Reduce duplication with ArticleResource::put?

        use chrono::{TimeZone, Local};
        use futures::Stream;

        #[derive(BartDisplay)]
        #[template="templates/article_contents.html"]
        struct Template<'a> {
            title: &'a str,
            rendered: String,
        }

        #[derive(Serialize)]
        struct PutResponse<'a> {
            slug: &'a str,
            article_id: i32,
            revision: i32,
            title: &'a str,
            body: &'a str,
            rendered: &'a str,
            last_updated: &'a str,
        }

        Box::new(body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |arg: CreateArticle| {
                if arg.base_revision != NEW {
                    unimplemented!("Version update conflict");
                }
                self.state.create_article(self.slug.clone(), arg.title, arg.body, identity)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::Ok)
                    .with_header(ContentType(APPLICATION_JSON.clone()))
                    .with_body(serde_json::to_string(&PutResponse {
                        slug: &updated.slug,
                        article_id: updated.article_id,
                        revision: updated.revision,
                        title: &updated.title,
                        body: &updated.body,
                        rendered: &Template {
                            title: &updated.title,
                            rendered: render_markdown(&updated.body),
                        }.to_string(),
                        last_updated: &super::article_resource::last_updated(
                            updated.article_id,
                            &Local.from_utc_datetime(&updated.created),
                            updated.author.as_ref().map(|x| &**x)
                        ),
                    }).expect("Should never fail"))
                )
            })
        )
    }

    fn post(self: Box<Self>, body: hyper::Body, identity: Option<String>) -> ResponseFuture {
        // TODO Check incoming Content-Type
        // TODO Refactor? Reduce duplication with ArticleResource::put?

        use futures::Stream;

        Box::new(body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |arg: CreateArticle| {
                if arg.base_revision != NEW {
                    unimplemented!("Version update conflict");
                }
                self.state.create_article(self.slug.clone(), arg.title, arg.body, identity)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::SeeOther)
                    .with_header(ContentType(TEXT_PLAIN.clone()))
                    .with_header(Location::new(updated.link().to_owned()))
                    .with_body("See other")
                )
            })
        )
    }
}
