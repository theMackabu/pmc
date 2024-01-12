mod helpers;
mod routes;
mod structs;

use crate::webui;
use bytes::Bytes;
use helpers::create_status;
use lazy_static::lazy_static;
use macros_rs::{crashln, fmtstr, str};
use pmc::{config, config::structs::Servers, process};
use prometheus::{opts, register_counter, register_gauge, register_histogram, register_histogram_vec};
use prometheus::{Counter, Gauge, Histogram, HistogramVec};
use serde_json::{json, Value};
use structs::{AuthMessage, ErrorMessage};

use static_dir::static_dir;
use std::{convert::Infallible, str::FromStr};
use utoipa_rapidoc::RapiDoc;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi, ToSchema,
};

use rocket::{
    catch,
    http::{ContentType, Status},
    outcome::Outcome,
    serde::{json::Json, Serialize},
    Request,
};

// #[inline]
// async fn convert_to_string(bytes: Bytes) -> Result<String, Rejection> { String::from_utf8(bytes.to_vec()).map_err(|_| reject()) }
//
// #[inline]
// fn string_filter(limit: u64) -> impl Filter<Extract = (String,), Error = Rejection> + Clone { body::content_length_limit(limit).and(body::bytes()).and_then(convert_to_string) }

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
struct SecurityAddon;

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
    const DOCS: &str = include_str!("docs/index.html");

    let s_path = config::read().get_path().trim_end_matches('/').to_string();

    let docs_path = fmtstr!("{}/docs.json", s_path);

    //
    //     let cors = warp::cors().allow_origin("*").allow_methods(vec!["GET", "POST"]).allow_headers(vec!["token"]);

    let tmpl = match webui::create_template_filter() {
        Ok(template) => template,
        Err(err) => crashln!("{err}"),
    };

    //
    //     let daemon_dump = path!("daemon" / "dump").and(get()).and_then(routes::dump_handler);
    //     let daemon_config = path!("daemon" / "config").and(get()).and_then(routes::config_handler);
    //     let daemon_servers = path!("daemon" / "servers").and(get()).and_then(routes::servers_handler);
    //     let daemon_metrics = path!("daemon" / "metrics").and(get()).and_then(routes::metrics_handler);
    //     let daemon_prometheus = path!("daemon" / "prometheus").and(get()).and_then(routes::prometheus_handler);
    //

    //
    //     let process_list = path!("list").and(get()).and_then(routes::list_handler);
    //     let process_env = path!("process" / usize / "env").and(get()).and_then(routes::env_handler);
    //     let process_info = path!("process" / usize / "info").and(get()).and_then(routes::info_handler);
    //     let process_logs = path!("process" / usize / "logs" / String).and(get()).and_then(routes::log_handler);
    //     let process_raw_logs = path!("process" / usize / "logs" / String / "raw").and(get()).and_then(routes::log_handler_raw);
    //     let process_create = path!("process" / "create").and(post()).and(body::json()).and_then(routes::create_handler);
    //     let process_action = post().and(path!("process" / usize / "action")).and(body::json()).and_then(routes::action_handler);
    //     let process_rename = path!("process" / usize / "rename").and(post()).and(string_filter(1024 * 16)).and_then(routes::rename_handler);
    //
    //     let web_login = get().and(path!("login")).and(tmpl.clone()).and_then(routes::login);
    //     let web_dashboard = get().and(path::end()).and(tmpl.clone()).and_then(routes::dashboard);
    //     let web_view_process = get().and(path!("view" / usize)).and(tmpl.clone()).and_then(routes::view_process);
    //
    //     let log = warp::log::custom(|info| {
    //         log!(
    //             "[api] {} (method={}, status={}, ms={:?}, ver={:?})",
    //             info.path(),
    //             info.method(),
    //             info.status().as_u16(),
    //             info.elapsed(),
    //             info.version()
    //         )
    //     });
    //
    //     let base = s_path
    //         .split('/')
    //         .enumerate()
    //         .filter(|(_, p)| !p.is_empty() || *p == s_path)
    //         .fold(warp::any().boxed(), |f, (_, path)| f.and(warp::path(path.to_owned())).boxed());
    //
    //     let routes = process_list
    //         .or(process_env)
    //         .or(process_info)
    //         .or(process_logs)
    //         .or(process_raw_logs)
    //         .or(process_create)
    //         .or(process_action)
    //         .or(process_rename)
    //         .or(daemon_dump)
    //         .or(daemon_config)
    //         .or(daemon_servers)
    //         .or(daemon_metrics)
    //         .or(daemon_prometheus);
    //
    //     let use_routes_basic = || async {
    //         let base_route = path::end().map(|| json(&json!({"healthy": true})).into_response());
    //         let internal = routes.clone().and(auth).or(root_redirect()).or(base_route).or(docs_json).or(docs_view).boxed();
    //         serve(base.clone().and(internal).recover(handle_rejection).with(log)).run(config::read().get_address()).await
    //     };
    //
    //     let use_routes_web = || async {
    //         let web_routes = web_login.or(web_dashboard).or(web_view_process).or(static_dir!("src/webui/assets"));
    //         let internal = routes.clone().and(auth).or(root_redirect()).or(web_routes).or(docs_json).or(docs_view).boxed();
    //         serve(base.clone().and(internal).recover(handle_rejection).with(log)).run().await
    //     };

    // match webui {
    //     true => use_routes_web().await,
    //     false => use_routes_basic().await,
    // }

    // let docs_json = path!("docs.json").and(get()).map(|| json(&));
    // .mount("/", RapiDoc::new(docs_path).custom_html(DOCS).to_html().path("/rapidoc"))

    let rocket = rocket::custom(config::read().get_address())
        .mount(
            format!("{s_path}/"),
            rocket::routes![
                docs,
                docs_json,
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
            ],
        )
        .register(format!("{s_path}/"), rocket::catchers![internal_error, not_allowed, not_found, unauthorized])
        .launch()
        .await;

    if let Err(err) = rocket {
        log::error!("failed to launch!\n{err}")
    }
}

#[rocket::get("/docs")]
pub async fn docs() -> (ContentType, String) {
    const DOCS: &str = include_str!("docs/index.html");

    let s_path = config::read().get_path().trim_end_matches('/').to_string();
    let docs_path = fmtstr!("{}/docs.json", s_path);

    (ContentType::HTML, RapiDoc::new(docs_path).custom_html(DOCS).to_html())
}

#[rocket::get("/docs.json")]
pub async fn docs_json() -> Value { json!(ApiDoc::openapi()) }
