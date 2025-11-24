mod docs;
mod fairing;
mod helpers;
mod routes;
mod structs;

use crate::webui::{self, assets::NamedFile};
use helpers::{NotFound, create_status};
use include_dir::{Dir, include_dir};
use lazy_static::lazy_static;
use pmc::{config, process};
use prometheus::{Counter, Gauge, Histogram, HistogramVec};
use prometheus::{
    opts, register_counter, register_gauge, register_histogram, register_histogram_vec,
};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use structs::ErrorMessage;
use tera::Context;

use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};

use rocket::{
    State, catch,
    http::{ContentType, Status},
    outcome::Outcome,
    request::{self, FromRequest, Request},
    serde::json::Json,
};

lazy_static! {
    pub static ref HTTP_COUNTER: Counter = register_counter!(opts!(
        "http_requests_total",
        "Number of HTTP requests made."
    ))
    .unwrap();
    pub static ref DAEMON_START_TIME: Gauge = register_gauge!(opts!(
        "process_start_time_seconds",
        "The uptime of the daemon."
    ))
    .unwrap();
    pub static ref DAEMON_MEM_USAGE: Histogram = register_histogram!(
        "daemon_memory_usage",
        "The memory usage graph of the daemon."
    )
    .unwrap();
    pub static ref DAEMON_CPU_PERCENTAGE: Histogram = register_histogram!(
        "daemon_cpu_percentage",
        "The cpu usage graph of the daemon."
    )
    .unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["route"]
    )
    .unwrap();
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
        routes::remote_list,
        routes::remote_info,
        routes::remote_metrics,
        routes::remote_logs,
        routes::remote_rename,
        routes::remote_action,
        routes::logs_raw_handler,
        routes::metrics_handler,
        routes::prometheus_handler,
        routes::create_handler,
        routes::rename_handler
    ),
    components(schemas(
        ErrorMessage,
        process::Log,
        process::Raw,
        process::Info,
        process::Stats,
        process::Watch,
        process::ItemSingle,
        process::ProcessItem,
        routes::Stats,
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
struct AddCORS;
struct EnableWebUI;
struct SecurityAddon;

struct TeraState {
    path: String,
    tera: tera::Tera,
}

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("token"))),
        )
    }
}

#[catch(500)]
fn internal_error<'m>() -> Json<ErrorMessage> {
    create_status(Status::InternalServerError)
}

#[catch(400)]
fn bad_request<'m>() -> Json<ErrorMessage> {
    create_status(Status::BadRequest)
}

#[catch(405)]
fn not_allowed<'m>() -> Json<ErrorMessage> {
    create_status(Status::MethodNotAllowed)
}

#[catch(404)]
fn not_found<'m>() -> Json<ErrorMessage> {
    create_status(Status::NotFound)
}

#[catch(401)]
fn unauthorized<'m>() -> Json<ErrorMessage> {
    create_status(Status::Unauthorized)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for EnableWebUI {
    type Error = ();

    async fn from_request(_req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let webui = IS_WEBUI.load(Ordering::Acquire);

        if webui {
            Outcome::Success(EnableWebUI)
        } else {
            Outcome::Error((rocket::http::Status::NotFound, ()))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for routes::Token {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let config = config::read().daemon.web;

        match config.secure {
            Some(val) => {
                if !val.enabled {
                    return Outcome::Success(routes::Token);
                }

                let header_valid = request
                    .headers()
                    .get_one("token")
                    .is_some_and(|header_value| header_value == val.token);
                let query_valid = match request.query_value::<String>("token") {
                    Some(Ok(query_token)) => query_token == val.token,
                    _ => false,
                };

                if header_valid || query_valid {
                    return Outcome::Success(routes::Token);
                }

                Outcome::Error((rocket::http::Status::Unauthorized, ()))
            }
            None => return Outcome::Success(routes::Token),
        }
    }
}

static IS_WEBUI: AtomicBool = AtomicBool::new(false);

pub async fn start(webui: bool) {
    IS_WEBUI.store(webui, Ordering::Release);

    let tera = webui::create_templates();
    let s_path = config::read().get_path().trim_end_matches('/').to_string();

    let routes = rocket::routes![
        embed,
        scalar,
        health,
        docs_json,
        static_assets,
        dynamic_assets,
        routes::login,
        routes::servers,
        routes::dashboard,
        routes::view_process,
        routes::server_status,
        routes::action_handler,
        routes::env_handler,
        routes::info_handler,
        routes::dump_handler,
        routes::remote_list,
        routes::remote_info,
        routes::remote_logs,
        routes::remote_rename,
        routes::remote_action,
        routes::servers_handler,
        routes::config_handler,
        routes::list_handler,
        routes::logs_handler,
        routes::logs_raw_handler,
        routes::logs_ws,
        routes::metrics_handler,
        routes::remote_metrics,
        routes::stream_info,
        routes::stream_metrics,
        routes::prometheus_handler,
        routes::create_handler,
        routes::rename_handler,
        routes::remote_logs_ws,
    ];

    let rocket = rocket::custom(config::read().get_address())
        .attach(Logger)
        .attach(AddCORS)
        .manage(TeraState {
            path: tera.1,
            tera: tera.0,
        })
        .mount(format!("{s_path}/"), routes)
        .register(
            "/",
            rocket::catchers![
                internal_error,
                bad_request,
                not_allowed,
                not_found,
                unauthorized
            ],
        )
        .launch()
        .await;

    if let Err(err) = rocket {
        log::error!("failed to launch!\n{err}")
    }
}

async fn render(
    name: &str,
    state: &State<TeraState>,
    ctx: &mut Context,
) -> Result<String, NotFound> {
    ctx.insert("base_path", &state.path);
    ctx.insert("build_version", env!("CARGO_PKG_VERSION"));

    state
        .tera
        .render(name, &ctx)
        .or(Err(helpers::not_found("Page was not found")))
}

#[rocket::get("/assets/<name>")]
async fn dynamic_assets(name: String) -> Option<NamedFile> {
    static DIR: Dir = include_dir!("src/webui/dist/assets");
    let file = DIR.get_file(&name)?;

    NamedFile::send(name, file.contents_utf8()).await.ok()
}

#[rocket::get("/static/<name>")]
async fn static_assets(name: String) -> Option<NamedFile> {
    static DIR: Dir = include_dir!("src/daemon/static");
    let file = DIR.get_file(&name)?;

    NamedFile::send(name, file.contents_utf8()).await.ok()
}

#[rocket::get("/openapi.json")]
async fn docs_json() -> Value {
    json!(ApiDoc::openapi())
}

#[rocket::get("/docs/embed")]
async fn embed() -> (ContentType, String) {
    (ContentType::HTML, docs::Docs::new().render())
}

#[rocket::get("/docs")]
async fn scalar(
    state: &State<TeraState>,
    _webui: EnableWebUI,
) -> Result<(ContentType, String), NotFound> {
    Ok((
        ContentType::HTML,
        render("docs", &state, &mut Context::new()).await?,
    ))
}

#[rocket::get("/health")]
async fn health() -> Value {
    json!({"healthy": true})
}
