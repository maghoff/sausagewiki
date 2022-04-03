pub mod pagination;

mod about_resource;
mod article_resource;
mod article_revision_resource;
mod changes_resource;
mod diff_resource;
mod html_resource;
mod new_article_resource;
mod read_only_resource;
mod search_resource;
mod sitemap_resource;
mod temporary_redirect_resource;

pub use self::about_resource::AboutResource;
pub use self::article_resource::ArticleResource;
pub use self::article_revision_resource::ArticleRevisionResource;
pub use self::changes_resource::{ChangesLookup, ChangesResource};
pub use self::diff_resource::{DiffLookup, DiffResource};
pub use self::html_resource::HtmlResource;
pub use self::new_article_resource::NewArticleResource;
pub use self::read_only_resource::ReadOnlyResource;
pub use self::search_resource::SearchLookup;
pub use self::sitemap_resource::SitemapResource;
pub use self::temporary_redirect_resource::TemporaryRedirectResource;
