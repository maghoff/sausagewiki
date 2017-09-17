use std::collections::HashMap;

use futures::{self, Future};

use assets::*;
use article_resource::ArticleResource;
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
    type Future = futures::BoxFuture<Option<Self::Resource>, Self::Error>;

    fn lookup(&self, path: &str, _query: Option<&str>, _fragment: Option<&str>) -> Self::Future {
        assert!(path.starts_with("/"));

        if path.starts_with("/_") {
            // Reserved namespace

            return futures::finished(
                LOOKUP_MAP.get(path).map(|x| x())
            ).boxed();
        }

        let mut split = path[1..].split('/');

        let slug = split.next().expect("Always at least one element");

        if split.next() != None {
            // Currently disallow any URLs of the form /slug/...
            return futures::finished(None).boxed();
        }

        if let Ok(article_id) = slug.parse() {
            let state = self.state.clone();
            self.state.get_article_revision_by_id(article_id)
                .and_then(|x| Ok(x.map(move |article|
                    Box::new(ArticleResource::new(state, article)) as Box<Resource + Sync + Send>
                )))
                .boxed()
        } else {
            futures::finished(None).boxed()
        }
    }
}
