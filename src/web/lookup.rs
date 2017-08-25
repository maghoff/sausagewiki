use super::resource;

use futures;

pub trait Lookup {
    type Resource: resource::Resource;
    type Error;
    type Future: futures::Future<Item=Option<Self::Resource>, Error=Self::Error>;

    fn lookup(&self, path: &str, query: Option<&str>, fragment: Option<&str>) -> Self::Future;
}
