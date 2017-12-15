#[macro_use] extern crate quote;
extern crate diesel_migrations;
extern crate diesel;
extern crate walkdir;

use diesel::Connection;
use diesel::prelude::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("cargo must set OUT_DIR");
    let db_path = Path::new(&out_dir).join("build.db");
    let db_path = db_path.to_str().expect("Will only work for Unicode-representable paths");

    let _ignore_failure = std::fs::remove_file(db_path);

    let connection = SqliteConnection::establish(db_path)
        .expect(&format!("Error esablishing a database connection to {}", db_path));

    // Integer is a dummy placeholder. Compiling fails when passing ().
    diesel::expression::sql_literal::sql::<(diesel::types::Integer)>("PRAGMA foreign_keys = ON")
        .execute(&connection)
        .expect("Should be able to enable foreign keys");

    diesel_migrations::run_pending_migrations(&connection).unwrap();

    let infer_schema_path = Path::new(&out_dir).join("infer_schema.rs");
    let mut file = File::create(infer_schema_path).expect("Unable to open file for writing");

    file.write_all(quote! {
        mod __diesel_infer_schema_articles {
            infer_table_from_schema!(#db_path, "articles");
        }
        pub use self::__diesel_infer_schema_articles::*;

        mod __diesel_infer_schema_article_revisions {
            infer_table_from_schema!(#db_path, "article_revisions");
        }
        pub use self::__diesel_infer_schema_article_revisions::*;
    }.as_str().as_bytes()).expect("Unable to write to file");

    for entry in WalkDir::new("migrations").into_iter().filter_map(|e| e.ok()) {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }

    // For build_config.rs
    for env_var in &[
        "CONTINUOUS_INTEGRATION",
        "TRAVIS_BRANCH",
        "TRAVIS_COMMIT",
    ] {
        println!("cargo:rerun-if-env-changed={}", env_var);
    }
}
