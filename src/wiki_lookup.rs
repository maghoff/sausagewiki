use std::collections::HashMap;

use futures::{Future, finished, failed};
use percent_encoding::percent_decode;

use resources::*;
use assets::*;
use state::State;
use web::{Lookup, Resource};

type BoxResource = Box<Resource + Sync + Send>;
type ResourceFn = Box<Fn(&State) -> BoxResource + Sync + Send>;

lazy_static! {
    static ref LOOKUP_MAP: HashMap<String, ResourceFn> = {
        let mut lookup_map = HashMap::new();

        lookup_map.insert(
            "_changes".to_string(),
            Box::new(|state: &State|
                // TODO Use query arguments to fill in the `before` parameter below
                Box::new(ChangesResource::new(state.clone(), None)) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            "_sitemap".to_string(),
            Box::new(|state: &State|
                Box::new(SitemapResource::new(state.clone())) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            "_new".to_string(),
            Box::new(|state: &State|
                Box::new(NewArticleResource::new(state.clone(), None)) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            format!("_assets/style-{}.css", StyleCss::checksum()),
            Box::new(|_: &State| Box::new(StyleCss) as BoxResource) as ResourceFn
        );

        lookup_map.insert(
            format!("_assets/script-{}.js", ScriptJs::checksum()),
            Box::new(|_: &State| Box::new(ScriptJs) as BoxResource) as ResourceFn
        );

        lookup_map.insert(
            format!("_assets/amatic-sc-v9-latin-regular.woff"),
            Box::new(|_: &State| Box::new(AmaticFont) as BoxResource) as ResourceFn
        );

        lookup_map
    };
}

#[derive(Clone)]
pub struct WikiLookup {
    state: State
}

fn split_one(path: &str) -> Result<(::std::borrow::Cow<str>, Option<&str>), ::std::str::Utf8Error> {
    let mut split = path.splitn(2, '/');
    let head = split.next().expect("At least one item must be returned");
    let head = percent_decode(head.as_bytes()).decode_utf8()?;
    let tail = split.next();

    Ok((head, tail))
}

impl WikiLookup {
    pub fn new(state: State) -> WikiLookup {
        WikiLookup { state }
    }

    fn reserved_lookup(&self, path: &str, _query: Option<&str>) -> <Self as Lookup>::Future {
        Box::new(finished(
            LOOKUP_MAP.get(path).map(|x| x(&self.state))
        ))
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

        // Normalize all user-generated slugs:
        let slugified_slug = ::slug::slugify(&slug);
        if slugified_slug != slug {
            return Box::new(finished(Some(
                Box::new(ArticleRedirectResource::new(slugified_slug)) as BoxResource
            )));
        }

        let state = self.state.clone();
        let edit = query == Some("edit");
        let slug = slug.into_owned();

        use state::SlugLookup;
        Box::new(self.state.lookup_slug(slug.clone())
            .and_then(move |x| Ok(Some(match x {
                SlugLookup::Miss =>
                    Box::new(NewArticleResource::new(state, Some(slug))) as BoxResource,
                SlugLookup::Hit { article_id, revision } =>
                    Box::new(ArticleResource::new(state, article_id, revision, edit)) as BoxResource,
                SlugLookup::Redirect(slug) =>
                    Box::new(ArticleRedirectResource::new(slug)) as BoxResource,
            })))
        )
    }
}

impl Lookup for WikiLookup {
    type Resource = BoxResource;
    type Error = Box<::std::error::Error + Send + Sync>;
    type Future = Box<Future<Item = Option<Self::Resource>, Error = Self::Error>>;

    fn lookup(&self, path: &str, query: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));
        let path = &path[1..];

        if path.starts_with("_") {
            self.reserved_lookup(path, query)
        } else {
            self.article_lookup(path, query)
        }
    }
}
