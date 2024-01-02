use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{string, ternary, then};
use prometheus::{Encoder, TextEncoder};
use psutil::process::{MemoryInfo, Process};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use tera::{Context, Tera};
use utoipa::ToSchema;

use pmc::{
    file, helpers,
    process::{dump, Runner},
};

use crate::daemon::{
    api::{HTTP_COUNTER, HTTP_REQ_HISTOGRAM},
    pid,
};

use warp::{
    hyper::body::Body,
    reject,
    reply::{self, json, Response},
    Rejection, Reply,
};

use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(as = MemoryInfo)]
pub(crate) struct DocMemoryInfo {
    rss: u64,
    vms: u64,
    #[cfg(target_os = "linux")]
    shared: u64,
    #[cfg(target_os = "linux")]
    text: u64,
    #[cfg(target_os = "linux")]
    data: u64,
    #[cfg(target_os = "macos")]
    page_faults: u64,
    #[cfg(target_os = "macos")]
    pageins: u64,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ActionBody {
    #[schema(example = "restart")]
    method: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateBody {
    #[schema(example = "app")]
    name: Option<String>,
    #[schema(example = "node index.js")]
    script: String,
    #[schema(example = "/projects/app")]
    path: PathBuf,
    #[schema(example = "src")]
    watch: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct ActionResponse<'a> {
    #[schema(example = true)]
    done: bool,
    #[schema(example = "name")]
    action: &'a str,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct LogResponse {
    logs: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct MetricsRoot<'a> {
    pub version: Version<'a>,
    pub daemon: Daemon,
}

#[derive(Serialize, ToSchema)]
pub struct Version<'a> {
    #[schema(example = "v1.0.0")]
    pub pkg: String,
    pub hash: &'a str,
    #[schema(example = "2000-01-01")]
    pub build_date: &'a str,
    #[schema(example = "release")]
    pub target: &'a str,
}

#[derive(Serialize, ToSchema)]
pub struct Daemon {
    pub pid: Option<i32>,
    #[schema(example = true)]
    pub running: bool,
    pub uptime: String,
    pub process_count: usize,
    #[schema(example = "default")]
    pub daemon_type: String,
    pub stats: Stats,
}

#[derive(Serialize, ToSchema)]
pub struct Stats {
    pub memory_usage: String,
    pub cpu_percent: String,
}

#[inline]
fn attempt(done: bool, method: &str) -> reply::Json {
    let data = json!(ActionResponse {
        done,
        action: ternary!(done, method, "DOES_NOT_EXIST")
    });

    json(&data)
}

#[inline]
fn render(name: &str, tmpl: &Tera, ctx: &Context) -> Result<String, Rejection> { tmpl.render(name, &ctx).or(Err(reject::not_found())) }

#[inline]
pub async fn login(store: (Tera, String)) -> Result<Box<dyn Reply>, Rejection> {
    let mut ctx = Context::new();
    let (tmpl, path) = store;

    ctx.insert("base_path", &path);
    let payload = render("login", &tmpl, &ctx)?;
    Ok(Box::new(reply::html(payload)))
}

#[inline]
pub async fn dashboard(store: (Tera, String)) -> Result<Box<dyn Reply>, Rejection> {
    let mut ctx = Context::new();
    let (tmpl, path) = store;

    ctx.insert("base_path", &path);
    let payload = render("dashboard", &tmpl, &ctx)?;
    Ok(Box::new(reply::html(payload)))
}

#[inline]
pub async fn view_process(id: usize, store: (Tera, String)) -> Result<Box<dyn Reply>, Rejection> {
    let mut ctx = Context::new();
    let (tmpl, path) = store;

    ctx.insert("base_path", &path);
    ctx.insert("process_id", &id);

    let payload = render("view", &tmpl, &ctx)?;
    Ok(Box::new(reply::html(payload)))
}

#[inline]
#[utoipa::path(get, tag = "Daemon", path = "/prometheus", responses((status = 200, description = "Get prometheus metrics", body = String)))]
pub async fn prometheus_handler() -> Result<impl Reply, Infallible> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::<u8>::new();
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(format!("{}", String::from_utf8(buffer.clone()).unwrap()))
}

#[inline]
#[utoipa::path(get, path = "/dump", tag = "Process", responses((status = 200, description = "Dump processes successfully", body = [u8])))]
pub async fn dump_handler() -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["dump"]).start_timer();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Ok(dump::raw())
}

#[inline]
#[utoipa::path(get, path = "/list", tag = "Process", responses((status = 200, description = "List processes successfully", body = [ProcessItem])))]
pub async fn list_handler() -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["list"]).start_timer();
    let data = Runner::new().json();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Ok(json(&data))
}

#[inline]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/logs/{kind}",
    params(
        ("id" = usize, Path, description = "Process id to get logs for", example = 0),
        ("kind" = String, Path, description = "Log output type", example = "out")
    ),
    responses(
        (status = 200, description = "Process logs of {type} fetched", body = LogResponse),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage)
    )
)]
pub async fn log_handler(id: usize, kind: String) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["log"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            let log_file = match kind.as_str() {
                "out" | "stdout" => global!("pmc.logs.out", item.name.as_str()),
                "error" | "stderr" => global!("pmc.logs.error", item.name.as_str()),
                _ => global!("pmc.logs.out", item.name.as_str()),
            };

            match File::open(log_file) {
                Ok(data) => {
                    let reader = BufReader::new(data);
                    let logs: Vec<String> = reader.lines().collect::<io::Result<_>>().unwrap();

                    timer.observe_duration();
                    Ok(json(&json!(LogResponse { logs })))
                }
                Err(_) => Ok(json(&json!(LogResponse { logs: vec![] }))),
            }
        }
        None => {
            timer.observe_duration();
            Err(reject::not_found())
        }
    }
}

#[inline]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/logs/{kind}/raw",
    params(
        ("id" = usize, Path, description = "Process id to get logs for", example = 0),
        ("kind" = String, Path, description = "Log output type", example = "out")
    ),
    responses(
        (status = 200, description = "Process logs of {type} fetched raw", body = String),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage)
    )
)]
pub async fn log_handler_raw(id: usize, kind: String) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["log"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            let log_file = match kind.as_str() {
                "out" | "stdout" => global!("pmc.logs.out", item.name.as_str()),
                "error" | "stderr" => global!("pmc.logs.error", item.name.as_str()),
                _ => global!("pmc.logs.out", item.name.as_str()),
            };

            let data = match fs::read_to_string(log_file) {
                Ok(data) => data,
                Err(err) => err.to_string(),
            };

            timer.observe_duration();
            Ok(Response::new(Body::from(data)))
        }
        None => {
            timer.observe_duration();
            Err(reject::not_found())
        }
    }
}

#[inline]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/info",
    params(("id" = usize, Path, description = "Process id to get information for", example = 0)),
    responses(
        (status = 200, description = "Current process info retrieved", body = ItemSingle),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage)
    )
)]
pub async fn info_handler(id: usize) -> Result<impl Reply, Rejection> {
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
#[utoipa::path(post, tag = "Process", path = "/process/create", request_body(content = String),
    responses(
        (status = 200, description = "Create process successful", body = ActionResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to create process", body = ErrorMessage)
    )
)]
pub async fn create_handler(body: CreateBody) -> Result<impl Reply, Rejection> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["create"]).start_timer();
    let mut runner = Runner::new();

    HTTP_COUNTER.inc();

    let name = match body.name {
        Some(name) => string!(name),
        None => string!(body.script.split_whitespace().next().unwrap_or_default()),
    };

    runner.start(&name, &body.script, body.path, &body.watch).save();
    timer.observe_duration();

    Ok(attempt(true, "create"))
}

#[inline]
#[utoipa::path(post, tag = "Process", path = "/process/{id}/rename", request_body(content = String),
    params(("id" = usize, Path, description = "Process id to rename", example = 0)),
    responses(
        (status = 200, description = "Rename process successful", body = ActionResponse),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage)
    )
)]
pub async fn rename_handler(id: usize, body: String) -> Result<impl Reply, Rejection> {
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
#[utoipa::path(get, tag = "Process", path = "/process/{id}/env",
    params(("id" = usize, Path, description = "Process id to fetch env from", example = 0)),
    responses(
        (status = 200, description = "Current process env", body = HashMap<String, String>),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage)
    )
)]
pub async fn env_handler(id: usize) -> Result<impl Reply, Rejection> {
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
#[utoipa::path(post, tag = "Process", path = "/process/{id}/action", request_body = ActionBody,
    params(("id" = usize, Path, description = "Process id to run action on", example = 0)),
    responses(
        (status = 200, description = "Run action on process successful", body = ActionResponse),
        (status = NOT_FOUND, description = "Process/action was not found", body = ErrorMessage)
    )
)]
pub async fn action_handler(id: usize, body: ActionBody) -> Result<impl Reply, Rejection> {
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
#[utoipa::path(get, tag = "Daemon", path = "/metrics", responses((status = 200, description = "Get daemon metrics", body = MetricsRoot)))]
pub async fn metrics_handler() -> Result<impl Reply, Infallible> {
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

    let response = json!(MetricsRoot {
        version: Version {
            pkg: format!("v{}", env!("CARGO_PKG_VERSION")),
            hash: env!("GIT_HASH_FULL"),
            build_date: env!("BUILD_DATE"),
            target: env!("PROFILE"),
        },
        daemon: Daemon {
            pid: pid,
            running: pid::exists(),
            uptime: uptime,
            process_count: runner.count(),
            daemon_type: global!("pmc.daemon.kind"),
            stats: Stats { memory_usage, cpu_percent }
        }
    });

    timer.observe_duration();
    Ok(json(&response))
}
