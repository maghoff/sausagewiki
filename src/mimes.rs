use hyper::mime;

lazy_static! {
    pub static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
    pub static ref TEXT_PLAIN: mime::Mime = "text/plain;charset=utf-8".parse().unwrap();
    pub static ref APPLICATION_JSON: mime::Mime = "application/json".parse().unwrap();
}
