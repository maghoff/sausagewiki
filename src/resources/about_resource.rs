use futures::{self, Future};

use hyper::header::ContentType;
use hyper::server::*;

use crate::build_config;
use crate::mimes::*;
use crate::site::system_page;
use crate::web::{Resource, ResponseFuture};

#[derive(Licenses)]
pub struct AboutResource;

impl AboutResource {
    pub fn new() -> Self {
        AboutResource
    }
}

enum License {
    Bsd3Clause,
    Mit,
    Mpl2,
    Ofl11,
}

impl License {
    fn link(&self) -> &'static str {
        use self::License::*;
        match self {
            &Bsd3Clause => "bsd-3-clause",
            &Mit => "mit",
            &Mpl2 => "mpl2",
            &Ofl11 => "sil-ofl-1.1",
        }
    }

    fn name(&self) -> &'static str {
        use self::License::*;
        match self {
            &Bsd3Clause => "BSD-3-Clause",
            &Mit => "MIT",
            &Mpl2 => "MPL2",
            &Ofl11 => "OFL-1.1",
        }
    }
}

struct LicenseInfo {
    name: &'static str,
    link: Option<&'static str>,
    copyright: &'static str,
    license: License,
}

#[derive(BartDisplay)]
#[template = "templates/about.html"]
struct Template<'a> {
    deps: &'a [LicenseInfo],
}

impl<'a> Template<'a> {
    fn version(&self) -> &str {
        &build_config::VERSION
    }
}

impl Resource for AboutResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(
            Response::new()
                .with_status(hyper::StatusCode::Ok)
                .with_header(ContentType(TEXT_HTML.clone())),
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        let head = self.head();

        Box::new(head.and_then(move |head| {
            Ok(head.with_body(
                system_page(
                    None, // Hmm, should perhaps accept `base` as argument
                    "About Sausagewiki",
                    Template {
                        deps: *LICENSE_INFOS,
                    },
                )
                .to_string(),
            ))
        }))
    }
}
