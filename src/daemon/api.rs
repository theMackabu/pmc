use crate::daemon::pid;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use global_placeholders::global;
use lazy_static::lazy_static;
use macros_rs::{fmtstr, string, ternary, then};
use pmc::{config, file, helpers, process::Runner};
use prometheus::{opts, register_counter, register_gauge, register_histogram, register_histogram_vec};
use prometheus::{Counter, Encoder, Gauge, Histogram, HistogramVec, TextEncoder};
use psutil::process::{MemoryInfo, Process};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;

use warp::{
    body, get,
    http::StatusCode,
    path, post, reject,
    reply::{self, json},
    Filter, Rejection, Reply,
};

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

#[derive(Deserialize)]
struct ActionBody {
    method: String,
}

#[derive(Serialize)]
struct ActionResponse<'a> {
    done: bool,
    action: &'a str,
}

#[inline]
async fn convert_to_string(bytes: Bytes) -> Result<String, Rejection> { String::from_utf8(bytes.to_vec()).map_err(|_| reject()) }

#[inline]
fn string_filter(limit: u64) -> impl Filter<Extract = (String,), Error = Rejection> + Clone { body::content_length_limit(limit).and(body::bytes()).and_then(convert_to_string) }

#[inline]
fn attempt(done: bool, method: &str) -> reply::Json {
    let data = json!(ActionResponse {
        done,
        action: ternary!(done, method, "DOES_NOT_EXIST")
    });

    json(&data)
}

lazy_static! {
    pub static ref HTTP_COUNTER: Counter = register_counter!(opts!("http_requests_total", "Number of HTTP requests made.")).unwrap();
    pub static ref DAEMON_START_TIME: Gauge = register_gauge!(opts!("process_start_time_seconds", "The uptime of the daemon.")).unwrap();
    pub static ref DAEMON_MEM_USAGE: Histogram = register_histogram!("daemon_memory_usage", "The memory usage graph of the daemon.").unwrap();
    pub static ref DAEMON_CPU_PERCENTAGE: Histogram = register_histogram!("daemon_cpu_percentage", "The cpu usage graph of the daemon.").unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!("http_request_duration_seconds", "The HTTP request latencies in seconds.", &["route"]).unwrap();
}

pub async fn start() {
    let config = config::read().daemon.api;

    let list = path!("list").and_then(list_handler);
    let metrics = path!("metrics").and_then(metrics_handler);
    let prometheus = path!("prometheus").and_then(prometheus_handler);

    let env = path!("process" / usize / "env").and_then(env_handler);
    let info = path!("process" / usize / "info").and_then(info_handler);
    let logs = path!("process" / usize / "logs" / String).and_then(log_handler);
    let action = path!("process" / usize / "action").and(body::json()).and_then(action_handler);
    let rename = path!("process" / usize / "rename").and(string_filter(1024 * 16)).and_then(rename_handler);

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
        .or(get().and(env))
        .or(get().and(list))
        .or(get().and(info))
        .or(get().and(logs))
        .or(get().and(metrics))
        .or(post().and(rename))
        .or(post().and(action))
        .or(get().and(prometheus));

    if config.secure.enabled {
        let auth = warp::header::exact("authorization", fmtstr!("token {}", config.secure.token));
        warp::serve(routes.and(auth).recover(handle_rejection).with(log)).run(config::read().get_address()).await
    } else {
        warp::serve(routes.recover(handle_rejection).with(log)).run(config::read().get_address()).await
    }
}

#[inline]
async fn prometheus_handler() -> Result<impl Reply, Infallible> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::<u8>::new();
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(format!("{}", String::from_utf8(buffer.clone()).unwrap()))
}

#[inline]
async fn list_handler() -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["list"]).start_timer();
    let data = Runner::new().json();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Ok(json(&data))
}

#[inline]
async fn log_handler(id: usize, kind: String) -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["log"]).start_timer();
    let data = Runner::new().json();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Ok(json(&data))
}

#[inline]
async fn info_handler(id: usize) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["info"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            timer.observe_duration();
            Ok(json(&item.clone().json()))
        }
        None => {
            timer.observe_duration();
            Err(reject::not_found())
        }
    }
}

#[inline]
async fn rename_handler(id: usize, body: String) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["rename"]).start_timer();
    let mut runner = Runner::new();

    HTTP_COUNTER.inc();
    if runner.exists(id) {
        let item = runner.get(id);
        item.rename(body.trim().replace("\n", ""));
        then!(item.running, item.restart());
        timer.observe_duration();
        Ok(attempt(true, "rename"))
    } else {
        timer.observe_duration();
        Err(reject::not_found())
    }
}

#[inline]
async fn env_handler(id: usize) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["env"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            timer.observe_duration();
            Ok(json(&item.clone().env))
        }
        None => {
            timer.observe_duration();
            Err(reject::not_found())
        }
    }
}

#[inline]
async fn action_handler(id: usize, body: ActionBody) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["action"]).start_timer();
    let mut runner = Runner::new();
    let method = body.method.as_str();

    HTTP_COUNTER.inc();
    match method {
        "start" | "restart" => {
            runner.get(id).restart();
            timer.observe_duration();
            Ok(attempt(true, method))
        }
        "stop" | "kill" => {
            runner.get(id).stop();
            timer.observe_duration();
            Ok(attempt(true, method))
        }
        "remove" | "delete" => {
            runner.remove(id);
            timer.observe_duration();
            Ok(attempt(true, method))
        }
        _ => {
            timer.observe_duration();
            Err(reject::not_found())
        }
    }
}

#[inline]
async fn metrics_handler() -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["metrics"]).start_timer();
    let mut pid: Option<i32> = None;
    let mut cpu_percent: Option<f32> = None;
    let mut uptime: Option<DateTime<Utc>> = None;
    let mut memory_usage: Option<MemoryInfo> = None;
    let mut runner: Runner = file::read_rmp(global!("pmc.dump"));

    HTTP_COUNTER.inc();
    if pid::exists() {
        if let Ok(process_id) = pid::read() {
            if let Ok(mut process) = Process::new(process_id as u32) {
                pid = Some(process_id);
                uptime = Some(pid::uptime().unwrap());
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }
        }
    }

    let memory_usage =
        match memory_usage {
            Some(usage) => helpers::format_memory(usage.rss()),
            None => string!("0b"),
        };

    let cpu_percent = match cpu_percent {
        Some(percent) => format!("{:.2}%", percent),
        None => string!("0%"),
    };

    let uptime = match uptime {
        Some(uptime) => helpers::format_duration(uptime),
        None => string!("none"),
    };

    let pid = match pid {
        Some(pid) => string!(pid),
        None => string!("n/a"),
    };

    let response = json!({
        "version": {
            "pkg": format!("v{}", env!("CARGO_PKG_VERSION")),
            "hash": env!("GIT_HASH"),
            "build_date": env!("BUILD_DATE"),
            "target": env!("PROFILE"),
        },
        "daemon": {
            "pid": pid,
            "running": pid::exists(),
            "uptime": uptime,
            "process_count": runner.count(),
            "daemon_type": global!("pmc.daemon.kind"),
            "stats": {
                "memory_usage":memory_usage,
                "cpu_percent": cpu_percent,
            }

        }
    });

    timer.observe_duration();
    Ok(json(&response))
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
        message = "UNHANDLED_REJECTION";
    }

    let json = json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(reply::with_status(json, code))
}
