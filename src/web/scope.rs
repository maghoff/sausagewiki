use futures;

pub trait Scope {
    type Resource;
    type Error;
    type Future: futures::Future<Item=Option<Self::Resource>, Error=Self::Error>;

    fn scope_lookup(&self, path: &str, query: Option<&str>) -> Self::Future;
}
