use futures;

pub trait Lookup {
    type Resource;
    type Error;
    type Future: futures::Future<Item=Option<Self::Resource>, Error=Self::Error>;

    fn lookup(&self, path: &str, query: Option<&str>) -> Self::Future;
}
