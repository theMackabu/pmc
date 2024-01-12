mod fairing;
mod helpers;
mod routes;
mod structs;

use crate::webui::{self, assets::NamedFile};
use helpers::create_status;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use macros_rs::fmtstr;
use pmc::{config, config::structs::Servers, process};
use prometheus::{opts, register_counter, register_gauge, register_histogram, register_histogram_vec};
use prometheus::{Counter, Gauge, Histogram, HistogramVec};
use serde_json::{json, Value};
use structs::{AuthMessage, ErrorMessage};
use utoipa_rapidoc::RapiDoc;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};

use rocket::{
    catch,
    http::{ContentType, Status},
    outcome::Outcome,
    serde::json::Json,
};

lazy_static! {
    pub static ref HTTP_COUNTER: Counter = register_counter!(opts!("http_requests_total", "Number of HTTP requests made.")).unwrap();
    pub static ref DAEMON_START_TIME: Gauge = register_gauge!(opts!("process_start_time_seconds", "The uptime of the daemon.")).unwrap();
    pub static ref DAEMON_MEM_USAGE: Histogram = register_histogram!("daemon_memory_usage", "The memory usage graph of the daemon.").unwrap();
    pub static ref DAEMON_CPU_PERCENTAGE: Histogram = register_histogram!("daemon_cpu_percentage", "The cpu usage graph of the daemon.").unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!("http_request_duration_seconds", "The HTTP request latencies in seconds.", &["route"]).unwrap();
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        routes::action_handler,
        routes::env_handler,
        routes::info_handler,
        routes::dump_handler,
        routes::servers_handler,
        routes::config_handler,
        routes::list_handler,
        routes::logs_handler,
        routes::logs_raw_handler,
        routes::metrics_handler,
        routes::prometheus_handler,
        routes::create_handler,
        routes::rename_handler
    ),
    components(schemas(
        Servers,
        AuthMessage,
        ErrorMessage,
        process::Log,
        process::Raw,
        process::Info,
        process::Stats,
        process::Watch,
        process::ItemSingle,
        process::ProcessItem,
        routes::Stats,
        routes::Server,
        routes::Daemon,
        routes::Version,
        routes::ActionBody,
        routes::ConfigBody,
        routes::CreateBody,
        routes::MetricsRoot,
        routes::LogResponse,
        routes::DocMemoryInfo,
        routes::ActionResponse,
    ))
)]

struct ApiDoc;

struct Logger;

struct SecurityAddon;

struct TeraState {
    path: String,
    tera: tera::Tera,
}

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme("api_key", SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("token"))))
    }
}

#[catch(500)]
fn internal_error<'m>() -> Json<ErrorMessage> { create_status(Status::InternalServerError) }

#[catch(405)]
fn not_allowed<'m>() -> Json<ErrorMessage> { create_status(Status::MethodNotAllowed) }

#[catch(404)]
fn not_found<'m>() -> Json<ErrorMessage> { create_status(Status::NotFound) }

#[catch(401)]
fn unauthorized<'m>() -> Json<ErrorMessage> { create_status(Status::Unauthorized) }

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for routes::Token {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let config = config::read().daemon.web;

        if !config.secure.enabled {
            return Outcome::Success(routes::Token);
        }

        if let Some(header_value) = request.headers().get_one("token") {
            if header_value == config.secure.token {
                return Outcome::Success(routes::Token);
            }
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}

pub async fn start(webui: bool) {
    let tera = webui::create_templates();
    let s_path = config::read().get_path().trim_end_matches('/').to_string();

    let mut routes = rocket::routes![
        index,
        assets,
        routes::login,
        routes::dashboard,
        routes::view_process,
        routes::action_handler,
        routes::env_handler,
        routes::info_handler,
        routes::dump_handler,
        routes::servers_handler,
        routes::config_handler,
        routes::list_handler,
        routes::logs_handler,
        routes::logs_raw_handler,
        routes::metrics_handler,
        routes::prometheus_handler,
        routes::create_handler,
        routes::rename_handler,
        docs_json,
        docs,
    ];

    if webui {
        routes.remove(0);
    } else {
        routes.remove(1);
        routes.remove(2);
        routes.remove(3);
        routes.remove(4);

        log::debug!("{:?} {:?} {:?} {:?}", routes[1], routes[2], routes[3], routes[4]);
    }

    let rocket = rocket::custom(config::read().get_address())
        .attach(Logger)
        .manage(TeraState { path: tera.1, tera: tera.0 })
        .mount(format!("{s_path}/"), routes)
        .register(format!("{s_path}/"), rocket::catchers![internal_error, not_allowed, not_found, unauthorized])
        .launch()
        .await;

    if let Err(err) = rocket {
        log::error!("failed to launch!\n{err}")
    }
}

#[rocket::get("/assets/<name>")]
pub async fn assets(name: String) -> Option<NamedFile> {
    static DIR: Dir = include_dir!("src/webui/dist/assets");
    let file = DIR.get_file(&name)?;

    NamedFile::send(name, file.contents_utf8()).await.ok()
}

#[rocket::get("/docs")]
pub async fn docs() -> (ContentType, String) {
    const DOCS: &str = include_str!("docs/index.html");

    let s_path = config::read().get_path().trim_end_matches('/').to_string();
    let docs_path = fmtstr!("{}/docs.json", s_path);

    (ContentType::HTML, RapiDoc::new(docs_path).custom_html(DOCS).to_html())
}

#[rocket::get("/")]
pub async fn index() -> Value { json!({"healthy": true}) }

#[rocket::get("/docs.json")]
pub async fn docs_json() -> Value { json!(ApiDoc::openapi()) }
