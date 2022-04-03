#![allow(dead_code)]

// The non-CARGO env variables used here must be listed
// in build.rs to properly trigger rebuild on change

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

const SOFT_HYPHEN: &str = "\u{00AD}";

#[cfg(all(not(debug_assertions), feature = "dynamic-assets"))]
compile_error!("dynamic-assets must not be used for production");

lazy_static! {
    pub static ref VERSION: String = || -> String {
        let mut components = vec![];

        #[cfg(debug_assertions)]
        components.push("debug".into());

        #[cfg(test)]
        components.push("test".into());

        #[cfg(feature = "dynamic-assets")]
        components.push("dynamic-assets".into());

        if option_env!("CONTINUOUS_INTEGRATION").is_none() {
            components.push("local-build".into());
        }

        if let Some(branch) = option_env!("TRAVIS_BRANCH") {
            components.push(format!("branch:{}", branch));
        }

        if let Some(commit) = option_env!("TRAVIS_COMMIT") {
            components.push(format!(
                "commit:{}",
                commit
                    .as_bytes()
                    .chunks(4)
                    .map(|x| String::from_utf8(x.to_owned()).unwrap_or_else(|_| String::new()))
                    .collect::<Vec<_>>()
                    .join(SOFT_HYPHEN)
            ));
        }

        if !components.is_empty() {
            format!("{} ({})", env!("CARGO_PKG_VERSION"), components.join(" "))
        } else {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }();
    pub static ref HTTP_SERVER: String = format!("{}/{}", PROJECT_NAME, VERSION.as_str());
}
