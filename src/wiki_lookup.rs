use std::collections::HashMap;

use futures::{Future, finished};

use assets::*;
use article_resource::ArticleResource;
use article_redirect_resource::ArticleRedirectResource;
use state::State;
use web::{Lookup, Resource};

lazy_static! {
    static ref LOOKUP_MAP: HashMap<String, Box<Fn() -> Box<Resource + Sync + Send> + Sync + Send>> = {
        let mut lookup_map = HashMap::new();

        lookup_map.insert(
            format!("/_assets/style-{}.css", StyleCss::checksum()),
            Box::new(|| Box::new(StyleCss) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send> + Sync + Send>
        );

        lookup_map.insert(
            format!("/_assets/script-{}.js", ScriptJs::checksum()),
            Box::new(|| Box::new(ScriptJs) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send> + Sync + Send>
        );

        lookup_map.insert(
            format!("/_assets/amatic-sc-v9-latin-regular.woff"),
            Box::new(|| Box::new(AmaticFont) as Box<Resource + Sync + Send>)
                as Box<Fn() -> Box<Resource + Sync + Send> + Sync + Send>
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
    type Resource = Box<Resource + Send + Sync>;
    type Error = Box<::std::error::Error + Send + Sync>;
    type Future = Box<Future<Item = Option<Self::Resource>, Error = Self::Error>>;

    fn lookup(&self, path: &str, _query: Option<&str>, _fragment: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));

        if path.starts_with("/_") {
            // Reserved namespace

            return Box::new(finished(
                LOOKUP_MAP.get(path).map(|x| x())
            ));
        }

        let mut split = path[1..].split('/');

        let slug = split.next().expect("Always at least one element");

        if split.next() != None {
            // Currently disallow any URLs of the form /slug/...
            return Box::new(finished(None));
        }

        let state = self.state.clone();

        use state::SlugLookup;
        Box::new(self.state.lookup_slug(slug.to_owned())
            .and_then(|x| Ok(match x {
                SlugLookup::Miss => None,
                SlugLookup::Hit { article_id, revision } =>
                    Some(Box::new(ArticleResource::new(state, article_id, revision))
                        as Box<Resource + Sync + Send>),
                SlugLookup::Redirect(slug) =>
                    Some(Box::new(ArticleRedirectResource::new(slug))
                        as Box<Resource + Sync + Send>)
            })))
    }
}
