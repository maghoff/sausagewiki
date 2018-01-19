use pulldown_cmark::{Parser, html, OPTION_ENABLE_TABLES, OPTION_DISABLE_HTML};
use pulldown_cmark::Event::Text;

pub fn render_markdown(src: &str) -> String {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    let p = Parser::new_ext(src, opts);
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

pub fn render_markdown_for_fts(src: &str) -> String {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    let p = Parser::new_ext(src, opts);
    let mut buf = String::new();

    for event in p {
        match event {
            Text(text) => buf.push_str(&text),
            _ => buf.push_str(" "),
        }
    }

    buf.replace('&', "");
    buf.replace('<', "");
    buf.replace('>', "");

    buf
}
