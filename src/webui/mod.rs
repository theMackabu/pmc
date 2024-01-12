use pmc::config;
use tera::Tera;

pub fn create_templates() -> (Tera, String) {
    let mut tera = Tera::default();
    let path = config::read().get_path();

    tera.add_raw_templates(vec![
        ("view", include_str!("dist/view.html")),
        ("login", include_str!("dist/login.html")),
        ("dashboard", include_str!("dist/index.html")),
    ])
    .unwrap();

    return (tera, path.trim_end_matches('/').to_string());
}

pub mod assets;
