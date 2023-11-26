use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{string, ternary, then};
use pmc::{file, helpers, process::Runner};
use prometheus::{Encoder, TextEncoder};
use psutil::process::{MemoryInfo, Process};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;

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
};

#[derive(Deserialize)]
pub struct ActionBody {
    method: String,
}

#[derive(Serialize)]
struct ActionResponse<'a> {
    done: bool,
    action: &'a str,
}

#[derive(Serialize)]
struct LogResponse {
    logs: Vec<String>,
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
pub async fn prometheus_handler() -> Result<impl Reply, Infallible> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::<u8>::new();
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(format!("{}", String::from_utf8(buffer.clone()).unwrap()))
}

#[inline]
pub async fn list_handler() -> Result<impl Reply, Infallible> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["list"]).start_timer();
    let data = Runner::new().json();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Ok(json(&data))
}

#[inline]
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
