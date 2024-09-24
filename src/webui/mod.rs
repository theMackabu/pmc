use pmc::config;
use tera::Tera;

pub fn create_templates() -> (Tera, String) {
    let mut tera = Tera::default();
    let path = config::read().get_path();

    tera.add_raw_templates(vec![
        ("docs", include_str!("dist/docs.html")),
        ("view", include_str!("dist/view.html")),
        ("login", include_str!("dist/login.html")),
        ("dashboard", include_str!("dist/index.html")),
        ("status", include_str!("dist/status.html")),
        ("servers", include_str!("dist/servers.html")),
    ])
    .unwrap();

    return (tera, path.trim_end_matches('/').to_string());
}

pub mod assets;
