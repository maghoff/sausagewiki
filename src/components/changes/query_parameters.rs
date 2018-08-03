use serde_urlencoded;

use pagination::Pagination;

pub const DEFAULT_LIMIT: i32 = 30;

#[derive(Serialize, Deserialize, Default)]
pub struct QueryParameters {
    pub after: Option<i32>,
    pub before: Option<i32>,

    pub article_id: Option<i32>,
    pub author: Option<String>,

    pub limit: Option<i32>,
}

impl QueryParameters {
    pub fn pagination(self, pagination: Pagination<i32>) -> Self {
        Self {
            after: if let Pagination::After(x) = pagination { Some(x) } else { None },
            before: if let Pagination::Before(x) = pagination { Some(x) } else { None },
            ..self
        }
    }

    pub fn article_id(self, article_id: Option<i32>) -> Self {
        Self { article_id, ..self }
    }

    pub fn author(self, author: Option<String>) -> Self {
        Self { author, ..self }
    }

    pub fn limit(self, limit: i32) -> Self {
        Self {
            limit: if limit != DEFAULT_LIMIT { Some(limit) } else { None },
            ..self
        }
    }

    pub fn into_link(self) -> String {
        let args = serde_urlencoded::to_string(self).expect("Serializing to String cannot fail");
        if args.len() > 0 {
            format!("?{}", args)
        } else {
            "_changes".to_owned()
        }
    }
}
