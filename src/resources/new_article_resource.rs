use futures::{self, Future};
use hyper;
use hyper::header::{ContentType, Location};
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use crate::assets::ScriptJs;
use crate::mimes::*;
use crate::rendering::render_markdown;
use crate::site::Layout;
use crate::state::State;
use crate::theme::{self, Theme};
use crate::web::{Resource, ResponseFuture};

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
    edit: bool,
}

#[derive(Deserialize)]
struct CreateArticle {
    base_revision: String,
    title: String,
    body: String,
    theme: Option<Theme>,
}

impl NewArticleResource {
    pub fn new(state: State, slug: Option<String>, edit: bool) -> Self {
        Self { state, slug, edit }
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
        // TODO Remove duplication with article_resource.rs:
        struct SelectableTheme {
            theme: Theme,
            selected: bool,
        }

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
            themes: &'a [SelectableTheme],
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
                        theme: theme::Theme::Gray,
                        body: &Template {
                            revision: NEW,
                            last_updated: None,
                            edit: self.edit,
                            cancel_url: self.slug.as_ref().map(|x| &**x),
                            title: &title,
                            raw: "",
                            rendered: EMPTY_ARTICLE_MESSAGE,
                            themes: &theme::THEMES.iter().map(|&x| SelectableTheme {
                                theme: x,
                                selected: false,
                            }).collect::<Vec<_>>(),
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
            theme: Theme,
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
                let theme = arg.theme.unwrap_or_else(theme::random);
                self.state.create_article(self.slug.clone(), arg.title, arg.body, identity, theme)
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
                        theme: updated.theme,
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
                let theme = arg.theme.unwrap_or_else(theme::random);
                self.state.create_article(self.slug.clone(), arg.title, arg.body, identity, theme)
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
