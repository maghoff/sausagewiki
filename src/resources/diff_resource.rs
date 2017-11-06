use std;
use std::str::FromStr;

use diff;
use futures::{self, Future};
use futures::future::{done, finished};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_urlencoded;

use mimes::*;
use models::ArticleRevision;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

const NONE: &str = "none";

type BoxResource = Box<Resource + Sync + Send>;
type Error = Box<std::error::Error + Send + Sync>;

#[derive(Clone)]
pub enum ArticleRevisionReference {
    None,
    Some {
        article_id: u32,
        revision: u32,
    }
}

use std::num::ParseIntError;

pub enum ArticleRevisionReferenceParseError {
    SplitError,
    ParseIntError(ParseIntError),
}

use std::fmt;
impl fmt::Display for ArticleRevisionReferenceParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ArticleRevisionReferenceParseError::*;
        match self {
            &SplitError => write!(f, "invalid format, must contain one @"),
            &ParseIntError(ref r) => r.fmt(f)
        }
    }
}

impl From<ParseIntError> for ArticleRevisionReferenceParseError {
    fn from(x: ParseIntError) -> ArticleRevisionReferenceParseError {
        ArticleRevisionReferenceParseError::ParseIntError(x)
    }
}

impl FromStr for ArticleRevisionReference {
    type Err = ArticleRevisionReferenceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == NONE {
            return Ok(ArticleRevisionReference::None);
        }

        let items: Vec<&str> = s.split("@").collect();
        if items.len() != 2 {
            return Err(ArticleRevisionReferenceParseError::SplitError)
        }

        let article_id = items[0].parse::<u32>()?;
        let revision = items[1].parse::<u32>()?;

        Ok(ArticleRevisionReference::Some { article_id, revision })
    }
}

impl fmt::Display for ArticleRevisionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ArticleRevisionReference::None => write!(f, "{}", NONE),
            &ArticleRevisionReference::Some { article_id, revision } => {
                write!(f, "{}@{}", article_id, revision)
            }
        }
    }
}

use serde::{de, Deserialize, Deserializer};
impl<'de> Deserialize<'de> for ArticleRevisionReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

use serde::{Serialize, Serializer};
impl Serialize for ArticleRevisionReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone)]
pub struct DiffLookup {
    state: State,
}

#[derive(Serialize, Deserialize)]
pub struct QueryParameters {
    from: ArticleRevisionReference,
    to: ArticleRevisionReference,
}

impl QueryParameters {
    pub fn new(from: ArticleRevisionReference, to: ArticleRevisionReference) -> QueryParameters {
        QueryParameters { from, to }
    }

    pub fn into_link(self) -> String {
        format!("_diff?{}", serde_urlencoded::to_string(self).expect("Serializing to String cannot fail"))
    }
}

impl DiffLookup {
    pub fn new(state: State) -> DiffLookup {
        Self { state }
    }

    pub fn lookup(&self, query: Option<&str>) -> Box<Future<Item=Option<BoxResource>, Error=::web::Error>> {
        let state = self.state.clone();

        Box::new(done((|| -> Result<Option<BoxResource>, ::web::Error> {
            let params: QueryParameters = serde_urlencoded::from_str(query.unwrap_or(""))?;

            Ok(Some(Box::new(DiffResource::new(state, params.from, params.to))))
        }())))
    }
}

pub struct DiffResource {
    state: State,
    from: ArticleRevisionReference,
    to: ArticleRevisionReference,
}

impl DiffResource {
    pub fn new(state: State, from: ArticleRevisionReference, to: ArticleRevisionReference) -> Self {
        Self { state, from, to }
    }

    fn query_args(&self) -> QueryParameters {
        QueryParameters {
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }

    fn get_article_revision(&self, r: &ArticleRevisionReference) -> Box<Future<Item = Option<ArticleRevision>, Error = Error>> {
        match r {
            &ArticleRevisionReference::None => Box::new(finished(None)),
            &ArticleRevisionReference::Some { article_id, revision } => Box::new(
                self.state.get_article_revision(article_id as i32, revision as i32)
            ),
        }
    }
}

impl Resource for DiffResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template = "templates/diff.html"]
        struct Template<'a> {
            title: &'a [Diff<char>],
            lines: &'a [Diff<&'a str>],
        }

        #[derive(Default)]
        struct Diff<T: fmt::Display> {
            removed: Option<T>,
            same: Option<T>,
            added: Option<T>,
        }

        let from = self.get_article_revision(&self.from);
        let to = self.get_article_revision(&self.to);

        let head = self.head();

        Box::new(head.join3(from, to)
            .and_then(move |(head, from, to)| {
                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: "Difference",
                        body: &Template {
                            title: &diff::chars(
                                from.as_ref().map(|x| &*x.title).unwrap_or(""),
                                to.as_ref().map(|x| &*x.title).unwrap_or("")
                            )
                                .into_iter()
                                .map(|x| match x {
                                    diff::Result::Left(x) => Diff { removed: Some(x), ..Default::default() },
                                    diff::Result::Both(x, _) => Diff { same: Some(x), ..Default::default() },
                                    diff::Result::Right(x) => Diff { added: Some(x), ..Default::default() },
                                })
                                .collect::<Vec<_>>(),
                            lines: &diff::lines(
                                from.as_ref().map(|x| &*x.body).unwrap_or(""),
                                to.as_ref().map(|x| &*x.body).unwrap_or("")
                            )
                                .into_iter()
                                .map(|x| match x {
                                    diff::Result::Left(x) => Diff { removed: Some(x), ..Default::default() },
                                    diff::Result::Both(x, _) => Diff { same: Some(x), ..Default::default() },
                                    diff::Result::Right(x) => Diff { added: Some(x), ..Default::default() },
                                })
                                .collect::<Vec<_>>()
                        },
                    }.to_string()))
            }))
    }
}
