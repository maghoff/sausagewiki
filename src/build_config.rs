#![allow(dead_code)]

// The non-CARGO env variables used here must be listed
// in build.rs to properly trigger rebuild on change

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

const SOFT_HYPHEN: &str = "\u{00AD}";

lazy_static! {
    pub static ref VERSION: String = || -> String {
        let mut components = Vec::<String>::new();

        #[cfg(debug_assertions)]
        components.push("debug".into());

        #[cfg(test)]
        components.push("test".into());

        if let None = option_env!("CONTINUOUS_INTEGRATION") {
            components.push("local-build".into());
        }

        if let Some(branch) = option_env!("TRAVIS_BRANCH") {
            components.push(format!("branch:{}", branch));
        }

        if let Some(commit) = option_env!("TRAVIS_COMMIT") {
            components.push(format!("commit:{}",
                commit
                    .as_bytes()
                    .chunks(4)
                    .map(|x|
                        String::from_utf8(x.to_owned())
                            .unwrap_or_else(|_| String::new())
                    )
                    .collect::<Vec<_>>()
                    .join(SOFT_HYPHEN)
            ));
        }

        if components.len() > 0 {
            format!("{} ({})", env!("CARGO_PKG_VERSION"), components.join(" "))
        } else {
            env!("CARGO_PKG_VERSION").to_string()
        }
    }();

    pub static ref HTTP_SERVER: String =
        format!("{}/{}", PROJECT_NAME, VERSION.as_str());
}
