use pmc::config;
use std::borrow::Cow;

const INDEX: &str = include_str!("../static/index.html");

#[derive(Clone)]
pub struct Docs {
    html: Cow<'static, str>,
    s_path: String,
}

impl Docs {
    pub fn new() -> Self {
        let s_path = config::read().get_path().trim_end_matches('/').to_string();
        Self {
            s_path,
            html: Cow::Borrowed(INDEX),
        }
    }

    pub fn render(&self) -> String {
        self.html.replace("$s_path", &self.s_path)
    }
}
