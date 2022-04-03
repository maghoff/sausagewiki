use std::borrow::Cow;
use std::collections::HashMap;
use std::str::Utf8Error;

use futures::future::FutureResult;
use futures::{done, failed, finished, Future};
use percent_encoding::percent_decode;
use slug::slugify;

use crate::resources::*;
use crate::state::State;
use crate::web::{Lookup, Resource};

#[allow(unused)]
use crate::assets::*;

type BoxResource = Box<dyn Resource + Sync + Send>;
type ResourceFn = Box<dyn Fn() -> BoxResource + Sync + Send>;

lazy_static! {
    static ref LICENSES_MAP: HashMap<&'static str, ResourceFn> = hashmap! {
        "bsd-3-clause" => Box::new(|| Box::new(
            HtmlResource::new(Some("../"), "The 3-Clause BSD License", include_str!("licenses/bsd-3-clause.html"))
        ) as BoxResource) as ResourceFn,
        "gpl3" => Box::new(|| Box::new(
            HtmlResource::new(Some("../"), "GNU General Public License", include_str!("licenses/gpl3.html"))
        ) as BoxResource) as ResourceFn,
        "mit" => Box::new(|| Box::new(
            HtmlResource::new(Some("../"), "The MIT License", include_str!("licenses/mit.html"))
        ) as BoxResource) as ResourceFn,
        "mpl2" => Box::new(|| Box::new(
            HtmlResource::new(Some("../"), "Mozilla Public License Version 2.0", include_str!("licenses/mpl2.html"))
        ) as BoxResource) as ResourceFn,
        "sil-ofl-1.1" => Box::new(|| Box::new(
            HtmlResource::new(Some("../"), "SIL Open Font License", include_str!("licenses/sil-ofl-1.1.html"))
        ) as BoxResource) as ResourceFn,
    };
}

#[derive(Clone)]
pub struct WikiLookup {
    state: State,
    changes_lookup: ChangesLookup,
    diff_lookup: DiffLookup,
    search_lookup: SearchLookup,
}

fn split_one(path: &str) -> Result<(Cow<str>, Option<&str>), Utf8Error> {
    let mut split = path.splitn(2, '/');
    let head = split.next().expect("At least one item must be returned");
    let head = percent_decode(head.as_bytes()).decode_utf8()?;
    let tail = split.next();

    Ok((head, tail))
}

fn map_lookup(
    map: &HashMap<&str, ResourceFn>,
    path: &str,
) -> FutureResult<Option<BoxResource>, Box<dyn ::std::error::Error + Send + Sync>> {
    let (head, tail) = match split_one(path) {
        Ok(x) => x,
        Err(x) => return failed(x.into()),
    };

    if tail.is_some() {
        return finished(None);
    }

    match map.get(head.as_ref()) {
        Some(resource_fn) => finished(Some(resource_fn())),
        None => finished(None),
    }
}

#[allow(unused)]
fn fs_lookup(
    root: &str,
    path: &str,
) -> FutureResult<Option<BoxResource>, Box<dyn ::std::error::Error + Send + Sync>> {
    use std::fs::File;
    use std::io::prelude::*;

    let extension = path.rsplit_once('.').map(|x| x.1);

    let content_type = match extension {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("woff") => "application/font-woff",
        _ => "application/binary",
    }
    .parse()
    .unwrap();

    let mut filename = root.to_string();
    filename.push_str(path);

    let mut f = File::open(&filename).unwrap_or_else(|_| panic!("Not found: {}", filename));

    let mut body = Vec::new();
    f.read_to_end(&mut body).expect("Unable to read file");

    finished(Some(Box::new(ReadOnlyResource { content_type, body })))
}

impl WikiLookup {
    pub fn new(state: State, show_authors: bool) -> WikiLookup {
        let changes_lookup = ChangesLookup::new(state.clone(), show_authors);
        let diff_lookup = DiffLookup::new(state.clone());
        let search_lookup = SearchLookup::new(state.clone());

        WikiLookup {
            state,
            changes_lookup,
            diff_lookup,
            search_lookup,
        }
    }

    fn revisions_lookup(&self, path: &str, _query: Option<&str>) -> <Self as Lookup>::Future {
        let (article_id, revision): (i32, i32) = match (|| -> Result<_, <Self as Lookup>::Error> {
            let (article_id, tail) = split_one(path)?;
            let (revision, tail) = split_one(tail.ok_or("Not found")?)?;
            if tail.is_some() {
                return Err("Not found".into());
            }

            Ok((article_id.parse()?, revision.parse()?))
        })() {
            Ok(x) => x,
            Err(_) => return Box::new(finished(None)),
        };

        Box::new(
            self.state
                .get_article_revision(article_id, revision)
                .and_then(|article_revision| {
                    Ok(article_revision
                        .map(move |x| Box::new(ArticleRevisionResource::new(x)) as BoxResource))
                }),
        )
    }

    fn by_id_lookup(&self, path: &str, _query: Option<&str>) -> <Self as Lookup>::Future {
        let article_id: i32 = match (|| -> Result<_, <Self as Lookup>::Error> {
            let (article_id, tail) = split_one(path)?;
            if tail.is_some() {
                return Err("Not found".into());
            }

            Ok(article_id.parse()?)
        })() {
            Ok(x) => x,
            Err(_) => return Box::new(finished(None)),
        };

        Box::new(self.state.get_article_slug(article_id).and_then(|slug| {
            Ok(slug.map(|slug| {
                Box::new(TemporaryRedirectResource::new(format!("../{}", slug))) as BoxResource
            }))
        }))
    }

    fn diff_lookup_f(&self, path: &str, query: Option<&str>) -> <Self as Lookup>::Future {
        let article_id: u32 = match (|| -> Result<_, <Self as Lookup>::Error> {
            let (article_id, tail) = split_one(path)?;
            if tail.is_some() {
                return Err("Not found".into());
            }

            Ok(article_id.parse()?)
        })() {
            Ok(x) => x,
            Err(_) => return Box::new(finished(None)),
        };

        Box::new(self.diff_lookup.lookup(article_id, query))
    }

    fn reserved_lookup(&self, path: &str, query: Option<&str>) -> <Self as Lookup>::Future {
        let (head, tail) = match split_one(path) {
            Ok(x) => x,
            Err(x) => return Box::new(failed(x.into())),
        };

        match (head.as_ref(), tail) {
            ("_about", None) => Box::new(finished(Some(
                Box::new(AboutResource::new()) as BoxResource
            ))),
            ("_about", Some(license)) => Box::new(map_lookup(&LICENSES_MAP, license)),
            #[cfg(feature = "dynamic-assets")]
            ("_assets", Some(asset)) => Box::new(fs_lookup(
                concat!(env!("CARGO_MANIFEST_DIR"), "/assets/"),
                asset,
            )),
            #[cfg(not(feature = "dynamic-assets"))]
            ("_assets", Some(asset)) => Box::new(map_lookup(&ASSETS_MAP, asset)),
            ("_by_id", Some(tail)) => self.by_id_lookup(tail, query),
            ("_changes", None) => Box::new(self.changes_lookup.lookup(query)),
            ("_diff", Some(tail)) => self.diff_lookup_f(tail, query),
            ("_new", None) => Box::new(finished(Some(Box::new(NewArticleResource::new(
                self.state.clone(),
                None,
                true,
            )) as BoxResource))),
            ("_revisions", Some(tail)) => self.revisions_lookup(tail, query),
            ("_search", None) => Box::new(done(self.search_lookup.lookup(query))),
            ("_sitemap", None) => Box::new(finished(Some(Box::new(SitemapResource::new(
                self.state.clone(),
            )) as BoxResource))),
            _ => Box::new(finished(None)),
        }
    }

    fn article_lookup(&self, path: &str, query: Option<&str>) -> <Self as Lookup>::Future {
        let (slug, tail) = match split_one(path) {
            Ok(x) => x,
            Err(x) => return Box::new(failed(x.into())),
        };

        if tail.is_some() {
            // Currently disallow any URLs of the form /slug/...
            return Box::new(finished(None));
        }

        let edit = query == Some("edit");

        // Normalize all user-generated slugs:
        let slugified_slug = slugify(&slug);
        if slugified_slug != slug {
            return Box::new(finished(Some(
                Box::new(TemporaryRedirectResource::from_slug(slugified_slug, edit)) as BoxResource,
            )));
        }

        let state = self.state.clone();
        let slug = slug.into_owned();

        use crate::state::SlugLookup;
        Box::new(self.state.lookup_slug(slug.clone()).and_then(move |x| {
            Ok(Some(match x {
                SlugLookup::Miss => {
                    Box::new(NewArticleResource::new(state, Some(slug), edit)) as BoxResource
                }
                SlugLookup::Hit {
                    article_id,
                    revision,
                } => {
                    Box::new(ArticleResource::new(state, article_id, revision, edit)) as BoxResource
                }
                SlugLookup::Redirect(slug) => {
                    Box::new(TemporaryRedirectResource::from_slug(slug, edit)) as BoxResource
                }
            }))
        }))
    }
}

impl Lookup for WikiLookup {
    type Resource = BoxResource;
    type Error = Box<dyn ::std::error::Error + Send + Sync>;
    type Future = Box<dyn Future<Item = Option<Self::Resource>, Error = Self::Error>>;

    fn lookup(&self, path: &str, query: Option<&str>) -> Self::Future {
        assert!(path.starts_with('/'));
        let path = &path[1..];

        if path.starts_with('_') {
            self.reserved_lookup(path, query)
        } else {
            self.article_lookup(path, query)
        }
    }
}
