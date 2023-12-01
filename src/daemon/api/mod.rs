mod routes;

use bytes::Bytes;
use lazy_static::lazy_static;
use macros_rs::fmtstr;
use pmc::{config, process};
use prometheus::{opts, register_counter, register_gauge, register_histogram, register_histogram_vec};
use prometheus::{Counter, Gauge, Histogram, HistogramVec};
use routes::{action_handler, env_handler, info_handler, list_handler, log_handler, log_handler_raw, metrics_handler, prometheus_handler, rename_handler};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use utoipa::{OpenApi, ToSchema};
use utoipa_rapidoc::RapiDoc;

use warp::{
    body, get,
    http::StatusCode,
    path, post, reject,
    reply::{self, html, json},
    Filter, Rejection, Reply,
};

#[derive(Serialize, ToSchema)]
struct ErrorMessage {
    #[schema(example = 404)]
    code: u16,
    #[schema(example = "NOT_FOUND")]
    message: String,
}

#[inline]
async fn convert_to_string(bytes: Bytes) -> Result<String, Rejection> { String::from_utf8(bytes.to_vec()).map_err(|_| reject()) }

#[inline]
fn string_filter(limit: u64) -> impl Filter<Extract = (String,), Error = Rejection> + Clone { body::content_length_limit(limit).and(body::bytes()).and_then(convert_to_string) }

lazy_static! {
    pub static ref HTTP_COUNTER: Counter = register_counter!(opts!("http_requests_total", "Number of HTTP requests made.")).unwrap();
    pub static ref DAEMON_START_TIME: Gauge = register_gauge!(opts!("process_start_time_seconds", "The uptime of the daemon.")).unwrap();
    pub static ref DAEMON_MEM_USAGE: Histogram = register_histogram!("daemon_memory_usage", "The memory usage graph of the daemon.").unwrap();
    pub static ref DAEMON_CPU_PERCENTAGE: Histogram = register_histogram!("daemon_cpu_percentage", "The cpu usage graph of the daemon.").unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!("http_request_duration_seconds", "The HTTP request latencies in seconds.", &["route"]).unwrap();
}

pub async fn start(webui: bool) {
    const DOCS: &str = include_str!("docs/index.html");
    let config = config::read().daemon.web;

    #[derive(OpenApi)]
    #[openapi(
        paths(
            routes::action_handler,
            routes::env_handler,
            routes::info_handler,
            routes::list_handler,
            routes::log_handler,
            routes::log_handler_raw,
            routes::metrics_handler,
            routes::prometheus_handler,
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
            routes::MetricsRoot,
            routes::LogResponse,
            routes::DocMemoryInfo,
            routes::ActionResponse,
        ))
    )]
    struct ApiDoc;

    let list = path!("list").and(get()).and_then(list_handler);
    let metrics = path!("metrics").and(get()).and_then(metrics_handler);
    let prometheus = path!("prometheus").and(get()).and_then(prometheus_handler);
    let file = path!("docs.json").and(get()).map(|| json(&ApiDoc::openapi()));
    let docs = path!("docs").and(get()).map(|| html(RapiDoc::new("/docs.json").custom_html(DOCS).to_html()));

    let env = path!("process" / usize / "env").and(get()).and_then(env_handler);
    let info = path!("process" / usize / "info").and(get()).and_then(info_handler);
    let logs = path!("process" / usize / "logs" / String).and(get()).and_then(log_handler);
    let raw_logs = path!("process" / usize / "logs" / String / "raw").and(get()).and_then(log_handler_raw);
    let action = path!("process" / usize / "action").and(post()).and(body::json()).and_then(action_handler);
    let rename = path!("process" / usize / "rename").and(post()).and(string_filter(1024 * 16)).and_then(rename_handler);

    let log = warp::log::custom(|info| {
        log!(
            "[api] {} (method={}, status={}, ms={:?}, ver={:?})",
            info.path(),
            info.method(),
            info.status().as_u16(),
            info.elapsed(),
            info.version()
        )
    });

    let routes = path::end()
        .map(|| json(&json!({"healthy": true})))
        .or(env)
        .or(docs)
        .or(file)
        .or(list)
        .or(info)
        .or(logs)
        .or(rename)
        .or(action)
        .or(metrics)
        .or(raw_logs)
        .or(prometheus);

    if config.secure.enabled {
        let auth = warp::header::exact("authorization", fmtstr!("token {}", config.secure.token));
        warp::serve(routes.and(auth).recover(handle_rejection).with(log)).run(config::read().get_address()).await
    } else {
        warp::serve(routes.recover(handle_rejection).with(log)).run(config::read().get_address()).await
    }
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    HTTP_COUNTER.inc();
    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<reject::MissingHeader>() {
        code = StatusCode::UNAUTHORIZED;
        message = "UNAUTHORIZED";
    } else if let Some(_) = err.find::<reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        log!("[api] unhandled rejection (err={:?})", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "INTERNAL_SERVER_ERROR";
    }

    let json = json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(reply::with_status(json, code))
}
