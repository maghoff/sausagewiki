use pulldown_cmark::Event::{End, Text};
use pulldown_cmark::{html, Parser, Tag, OPTION_DISABLE_HTML, OPTION_ENABLE_TABLES};
use slug::slugify;

fn slugify_link(text: &str, title: &str) -> Option<(String, String)> {
    Some((slugify(text), title.to_owned()))
}

fn parser(src: &str) -> Parser {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    Parser::new_with_broken_link_callback(src, opts, Some(&slugify_link))
}

pub fn render_markdown(src: &str) -> String {
    let p = parser(src);
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

fn is_html_special(c: char) -> bool {
    c == '&' || c == '<' || c == '>'
}

pub fn render_markdown_for_fts(src: &str) -> String {
    let p = parser(src);
    let mut buf = String::new();

    for event in p {
        match event {
            Text(text) => buf.push_str(&text.replace(is_html_special, " ")),
            End(Tag::Link(uri, _title)) => {
                buf.push_str(" (");
                buf.push_str(&uri.replace(is_html_special, " "));
                buf.push_str(") ");
            }
            _ => buf.push_str(" "),
        }
    }

    buf
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn slug_link() {
        let actual = render_markdown("[Slug link]");
        let expected = "<p><a href=\"slug-link\" title=\"Slug link\">Slug link</a></p>\n";
        assert_eq!(actual, expected);
    }

    #[test]
    fn footnote_links() {
        let actual = render_markdown("[Link]\n\n[Link]: target");
        let expected = "<p><a href=\"target\">Link</a></p>\n";
        assert_eq!(actual, expected);
    }
}
