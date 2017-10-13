use std::collections::HashMap;

use futures::{Future, finished, failed};
use percent_encoding::percent_decode;

use article_redirect_resource::ArticleRedirectResource;
use article_resource::ArticleResource;
use assets::*;
use new_article_resource::NewArticleResource;
use sitemap_resource::SitemapResource;
use state::State;
use web::{Lookup, Resource};

type BoxResource = Box<Resource + Sync + Send>;
type ResourceFn = Box<Fn(&State) -> BoxResource + Sync + Send>;

lazy_static! {
    static ref LOOKUP_MAP: HashMap<String, ResourceFn> = {
        let mut lookup_map = HashMap::new();

        use changes_resource::ChangesResource;

        lookup_map.insert(
            "/_changes".to_string(),
            Box::new(|state: &State|
                // TODO Use query arguments to fill in the `before` parameter below
                Box::new(ChangesResource::new(state.clone(), None)) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            "/_sitemap".to_string(),
            Box::new(|state: &State|
                Box::new(SitemapResource::new(state.clone())) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            "/_new".to_string(),
            Box::new(|state: &State|
                Box::new(NewArticleResource::new(state.clone(), None)) as BoxResource
            ) as ResourceFn
        );

        lookup_map.insert(
            format!("/_assets/style-{}.css", StyleCss::checksum()),
            Box::new(|_: &State| Box::new(StyleCss) as BoxResource) as ResourceFn
        );

        lookup_map.insert(
            format!("/_assets/script-{}.js", ScriptJs::checksum()),
            Box::new(|_: &State| Box::new(ScriptJs) as BoxResource) as ResourceFn
        );

        lookup_map.insert(
            format!("/_assets/amatic-sc-v9-latin-regular.woff"),
            Box::new(|_: &State| Box::new(AmaticFont) as BoxResource) as ResourceFn
        );

        lookup_map
    };
}

#[derive(Clone)]
pub struct WikiLookup {
    state: State
}

impl WikiLookup {
    pub fn new(state: State) -> WikiLookup {
        WikiLookup { state }
    }
}

impl Lookup for WikiLookup {
    type Resource = BoxResource;
    type Error = Box<::std::error::Error + Send + Sync>;
    type Future = Box<Future<Item = Option<Self::Resource>, Error = Self::Error>>;

    fn lookup(&self, path: &str, query: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));

        if path.starts_with("/_") {
            // Reserved namespace

            return Box::new(finished(
                LOOKUP_MAP.get(path).map(|x| x(&self.state))
            ));
        }

        let mut split = path[1..].split('/');

        let slug = split.next().expect("Always at least one element");
        let slug = match percent_decode(slug.as_bytes()).decode_utf8() {
            Ok(x) => x,
            Err(x) => return Box::new(failed(x.into()))
        }.to_string();

        if split.next() != None {
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
