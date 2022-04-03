#[cfg(not(feature = "dynamic-assets"))]
mod static_assets {
    use crate::web::{Resource, ResponseFuture};
    use futures::Future;
    use std::collections::HashMap;

    // The CSS should be built to a single CSS file at compile time
    #[derive(StaticResource)]
    #[filename = "assets/themes.css"]
    #[mime = "text/css"]
    pub struct ThemesCss;

    #[derive(StaticResource)]
    #[filename = "assets/style.css"]
    #[mime = "text/css"]
    pub struct StyleCss;

    #[derive(StaticResource)]
    #[filename = "assets/script.js"]
    #[mime = "application/javascript"]
    pub struct ScriptJs;

    #[derive(StaticResource)]
    #[filename = "assets/search.js"]
    #[mime = "application/javascript"]
    pub struct SearchJs;

    // SIL Open Font License 1.1: http://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL
    // Copyright 2015 The Amatic SC Project Authors (contact@sansoxygen.com)
    // #[derive(StaticResource)]
    // #[filename = "assets/amatic-sc-v9-latin-regular.woff"]
    // #[mime = "application/font-woff"]
    // pub struct AmaticFont;

    type BoxResource = Box<dyn Resource + Sync + Send>;
    type ResourceFn = Box<dyn Fn() -> BoxResource + Sync + Send>;
    lazy_static! {
        pub static ref ASSETS_MAP: HashMap<&'static str, ResourceFn> = hashmap!{
            // The CSS should be built to a single CSS file at compile time
            ThemesCss::resource_name() =>
                Box::new(|| Box::new(ThemesCss) as BoxResource) as ResourceFn,

            StyleCss::resource_name() =>
                Box::new(|| Box::new(StyleCss) as BoxResource) as ResourceFn,

            ScriptJs::resource_name() =>
                Box::new(|| Box::new(ScriptJs) as BoxResource) as ResourceFn,

            SearchJs::resource_name() =>
                Box::new(|| Box::new(SearchJs) as BoxResource) as ResourceFn,
        };
    }
}

#[cfg(not(feature = "dynamic-assets"))]
pub use self::static_assets::*;

#[cfg(feature = "dynamic-assets")]
mod dynamic_assets {
    pub struct ThemesCss;
    impl ThemesCss {
        pub fn resource_name() -> &'static str {
            "themes.css"
        }
    }

    pub struct StyleCss;
    impl StyleCss {
        pub fn resource_name() -> &'static str {
            "style.css"
        }
    }

    pub struct ScriptJs;
    impl ScriptJs {
        pub fn resource_name() -> &'static str {
            "script.js"
        }
    }

    pub struct SearchJs;
    impl SearchJs {
        pub fn resource_name() -> &'static str {
            "search.js"
        }
    }
}

#[cfg(feature = "dynamic-assets")]
pub use self::dynamic_assets::*;
