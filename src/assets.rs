use futures::Future;
use web::{Resource, ResponseFuture};

#[derive(StaticResource)]
#[filename = "assets/style.css"]
#[mime = "text/css"]
pub struct StyleCss;

#[derive(StaticResource)]
#[filename = "assets/script.js"]
#[mime = "application/javascript"]
pub struct ScriptJs;

// SIL Open Font License 1.1: http://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL
// Copyright 2015 The Amatic SC Project Authors (contact@sansoxygen.com)
#[derive(StaticResource)]
#[filename = "assets/amatic-sc-v9-latin-regular.woff"]
#[mime = "application/font-woff"]
pub struct AmaticFont;
