use pulldown_cmark::{Parser, html, OPTION_ENABLE_TABLES, OPTION_DISABLE_HTML};

pub fn render_markdown(src: &str) -> String {
    let opts = OPTION_ENABLE_TABLES | OPTION_DISABLE_HTML;
    let p = Parser::new_ext(src, opts);
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}
