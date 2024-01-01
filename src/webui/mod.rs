use pmc::config;
use std::error::Error;
use tera::Tera;
use warp::{filters, Filter};

pub fn create_template_filter() -> Result<filters::BoxedFilter<((Tera, String),)>, Box<dyn Error>> {
    let s_path = config::read().get_path();
    let mut tera = Tera::default();

    tera.add_raw_templates(vec![
        ("view", include_str!("dist/view.html")),
        ("login", include_str!("dist/login.html")),
        ("dashboard", include_str!("dist/index.html")),
    ])
    .unwrap();

    Ok(warp::any().map(move || (tera.clone(), s_path.trim_end_matches('/').to_string())).boxed())
}
