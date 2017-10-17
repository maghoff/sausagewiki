use std::{error, fmt};

use serde;
use serde_urlencoded;

#[derive(Debug)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", (self as &error::Error).description())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "`after` and `before` are mutually exclusive"
    }
}

#[derive(Deserialize)]
struct PaginationStruct<T> {
    after: Option<T>,
    before: Option<T>,
}

pub enum Pagination<T> {
    After(T),
    Before(T),
    None,
}

impl<T> PaginationStruct<T> {
    fn into_enum(self) -> Result<Pagination<T>, Error> {
        match (self.after, self.before) {
            (Some(x), None) => Ok(Pagination::After(x)),
            (None, Some(x)) => Ok(Pagination::Before(x)),
            (None, None) => Ok(Pagination::None),
            _ => Err(Error)
        }
    }
}

pub fn from_str<'a, T: serde::Deserialize<'a>>(s: &'a str) -> Result<Pagination<T>, Error> {
    let pagination: PaginationStruct<T> = serde_urlencoded::from_str(s).map_err(|_| Error)?; // TODO Proper error reporting
    Ok(pagination.into_enum()?)
}
