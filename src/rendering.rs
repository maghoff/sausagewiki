use pulldown_cmark::{Parser, Tag, html, OPTION_ENABLE_TABLES, OPTION_DISABLE_HTML};
use pulldown_cmark::Event::{Text, End};

pub fn render_markdown(src: &str) -> String {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    let p = Parser::new_ext(src, opts);
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

fn is_html_special(c: char) -> bool {
    c == '&' || c == '<' || c == '>'
}

pub fn render_markdown_for_fts(src: &str) -> String {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    let p = Parser::new_ext(src, opts);
    let mut buf = String::new();

    for event in p {
        match event {
            Text(text) =>
                buf.push_str(&text.replace(is_html_special, " ")),
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
