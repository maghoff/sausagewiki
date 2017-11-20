#![allow(dead_code)]

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

lazy_static! {
    pub static ref VERSION: String = || -> String {
        #[allow(unused_mut)]
        let mut components = Vec::<&'static str>::new();

        #[cfg(debug_assertions)]
        components.push("debug");

        if components.len() > 0 {
            format!("{} ({})", env!("CARGO_PKG_VERSION"), components.join(" "))
        } else {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }();

    pub static ref HTTP_SERVER: String =
        format!("{}/{}", PROJECT_NAME, VERSION.as_str());
}
