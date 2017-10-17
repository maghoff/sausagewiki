use serde;
use serde_urlencoded;

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
    fn into_enum(self) -> Pagination<T> {
        match (self.after, self.before) {
            (Some(x), _) => Pagination::After(x),
            (None, Some(x)) => Pagination::Before(x),
            _ => Pagination::None,
        }
    }
}

pub fn from_str<'a, T: serde::Deserialize<'a>>(s: &'a str) -> Result<Pagination<T>, serde::de::value::Error> {
    let pagination: PaginationStruct<T> = serde_urlencoded::from_str(s)?;
    Ok(pagination.into_enum())
}
