mod routes;

use crate::webui;
use bytes::Bytes;
use lazy_static::lazy_static;
use macros_rs::{crashln, fmtstr};
use pmc::{config, process};
use prometheus::{opts, register_counter, register_gauge, register_histogram, register_histogram_vec};
use prometheus::{Counter, Gauge, Histogram, HistogramVec};
use serde::Serialize;
use serde_json::json;
use static_dir::static_dir;
use std::{convert::Infallible, str::FromStr};

use utoipa::{OpenApi, ToSchema};
use utoipa_rapidoc::RapiDoc;

use warp::{
    body, filters, get, header,
    http::{StatusCode, Uri},
    path, post, redirect, reject,
    reply::{self, html, json},
    serve, Filter, Rejection, Reply,
};

use routes::{
    action_handler, dashboard, dump_handler, env_handler, info_handler, list_handler, log_handler, log_handler_raw, login, metrics_handler, prometheus_handler, rename_handler, view_process,
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
    let s_path = config::read().get_path();

    let docs_path = fmtstr!("{}/docs.json", s_path.trim_end_matches('/').to_string());
    let auth = header::exact("authorization", fmtstr!("token {}", config.secure.token));

    let tmpl =
        match webui::create_template_filter() {
            Ok(template) => template,
            Err(err) => crashln!("{err}"),
        };

    #[derive(OpenApi)]
    #[openapi(
        paths(
            routes::action_handler,
            routes::env_handler,
            routes::info_handler,
            routes::dump_handler,
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

    let app_dump = path!("dump").and(get()).and_then(dump_handler);
    let app_metrics = path!("metrics").and(get()).and_then(metrics_handler);
    let app_prometheus = path!("prometheus").and(get()).and_then(prometheus_handler);
    let app_docs_json = path!("docs.json").and(get()).map(|| json(&ApiDoc::openapi()));
    let app_docs = path!("docs").and(get()).map(|| html(RapiDoc::new(docs_path).custom_html(DOCS).to_html()));

    let process_list = path!("list").and(get()).and_then(list_handler);
    let process_env = path!("process" / usize / "env").and(get()).and_then(env_handler);
    let process_info = path!("process" / usize / "info").and(get()).and_then(info_handler);
    let process_logs = path!("process" / usize / "logs" / String).and(get()).and_then(log_handler);
    let process_raw_logs = path!("process" / usize / "logs" / String / "raw").and(get()).and_then(log_handler_raw);
    let process_action = path!("process" / usize / "action").and(post()).and(body::json()).and_then(action_handler);
    let process_rename = path!("process" / usize / "rename").and(post()).and(string_filter(1024 * 16)).and_then(rename_handler);

    let web_login = warp::get().and(path!("login")).and(tmpl.clone()).and_then(login);
    let web_dashboard = warp::get().and(path::end()).and(tmpl.clone()).and_then(dashboard);
    let web_view_process = warp::get().and(path!("view" / usize)).and(tmpl.clone()).and_then(view_process);

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

    let base = s_path
        .split('/')
        .enumerate()
        .filter(|(_, p)| !p.is_empty() || *p == s_path)
        .fold(warp::any().boxed(), |f, (_, path)| f.and(warp::path(path.to_owned())).boxed());

    let routes = process_list
        .or(process_env)
        .or(process_info)
        .or(process_logs)
        .or(process_raw_logs)
        .or(process_action)
        .or(process_rename)
        .or(app_metrics)
        .or(app_prometheus)
        .or(app_dump);

    let use_routes_basic = || async {
        let base_route = path::end().map(|| json(&json!({"healthy": true})).into_response());

        let internal = match config.secure.enabled {
            true => routes.clone().and(auth).or(root_redirect()).or(base_route).or(app_docs_json).or(app_docs).boxed(),
            false => routes.clone().or(root_redirect()).or(base_route).or(app_docs_json).or(app_docs).boxed(),
        };

        serve(base.clone().and(internal).recover(handle_rejection).with(log)).run(config::read().get_address()).await
    };

    let use_routes_web = || async {
        let web_routes = web_login.or(web_dashboard).or(web_view_process).or(static_dir!("src/webui/assets"));

        let internal = match config.secure.enabled {
            true => routes.clone().and(auth).or(root_redirect()).or(web_routes).or(app_docs_json).or(app_docs).boxed(),
            false => routes.clone().or(root_redirect()).or(web_routes).or(app_docs_json).or(app_docs).boxed(),
        };

        serve(base.clone().and(internal).recover(handle_rejection).with(log)).run(config::read().get_address()).await
    };

    match webui {
        true => use_routes_web().await,
        false => use_routes_basic().await,
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
    } else if let Some(_) = err.find::<reject::InvalidHeader>() {
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

fn root_redirect() -> filters::BoxedFilter<(impl Reply,)> {
    warp::path::full()
        .and_then(move |path: path::FullPath| async move {
            let path = path.as_str();

            if path.ends_with("/") || path.contains(".") {
                return Err(warp::reject());
            }

            Ok(redirect::redirect(Uri::from_str(&[path, "/"].concat()).unwrap()))
        })
        .boxed()
}
