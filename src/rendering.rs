use pulldown_cmark::{Event, Parser, html};

struct EscapeHtml<'a, I: Iterator<Item=Event<'a>>> {
    inner: I,
}

impl<'a, I: Iterator<Item=Event<'a>>> EscapeHtml<'a, I> {
    fn new(inner: I) -> EscapeHtml<'a, I> {
        EscapeHtml { inner }
    }
}

impl<'a, I: Iterator<Item=Event<'a>>> Iterator for EscapeHtml<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use pulldown_cmark::Event::{Text, Html, InlineHtml};

        match self.inner.next() {
            Some(Html(x)) => Some(Text(x)),
            Some(InlineHtml(x)) => Some(Text(x)),
            x => x
        }
    }
}

pub fn render_markdown(src: &str) -> String {
    let opts = ::pulldown_cmark::OPTION_ENABLE_TABLES;
    let p = EscapeHtml::new(Parser::new_ext(src, opts));
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}
