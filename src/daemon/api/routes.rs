#![allow(non_snake_case)]

use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{fmtstr, string, ternary, then};
use prometheus::{Encoder, TextEncoder};
use psutil::process::{MemoryInfo, Process};
use serde::Deserialize;
use tera::{Context, Tera};
use utoipa::ToSchema;

use rocket::{
    get,
    http::{ContentType, Status},
    post,
    serde::{json::Json, Serialize},
    State,
};

use super::{
    helpers::{generic_error, not_found, GenericError, NotFound},
    structs::ErrorMessage,
    EnableWebUI, TeraState,
};

use pmc::{
    config, file, helpers,
    process::{dump, http::client, ItemSingle, ProcessItem, Runner},
};

use crate::daemon::{
    api::{HTTP_COUNTER, HTTP_REQ_HISTOGRAM},
    pid,
};

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

pub(crate) struct Token;
type EnvList = Json<HashMap<String, String>>;

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

#[derive(Serialize, ToSchema)]
pub(crate) struct ConfigBody {
    #[schema(example = "bash")]
    shell: String,
    #[schema(min_items = 1, example = json!(["-c"]))]
    args: Vec<String>,
    #[schema(example = "/home/user/.pmc/logs")]
    log_path: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct CreateBody {
    #[schema(example = "app")]
    name: Option<String>,
    #[schema(example = "node index.js")]
    script: String,
    #[schema(value_type = String, example = "/projects/app")]
    path: PathBuf,
    #[schema(example = "src")]
    watch: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct ActionResponse {
    #[schema(example = true)]
    done: bool,
    #[schema(example = "name")]
    action: &'static str,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct LogResponse {
    logs: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct MetricsRoot {
    pub version: Version,
    pub daemon: Daemon,
}

#[derive(Serialize, ToSchema)]
pub struct Version {
    #[schema(example = "v1.0.0")]
    pub pkg: String,
    pub hash: &'static str,
    #[schema(example = "2000-01-01")]
    pub build_date: &'static str,
    #[schema(example = "release")]
    pub target: &'static str,
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

fn attempt(done: bool, method: &str) -> ActionResponse {
    ActionResponse {
        done,
        action: ternary!(done, Box::leak(Box::from(method)), "DOES_NOT_EXIST"),
    }
}

fn render(name: &str, tmpl: &Tera, ctx: &Context) -> Result<String, NotFound> { tmpl.render(name, &ctx).or(Err(not_found("Page was not found"))) }

#[get("/")]
pub async fn dashboard(state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> {
    let mut ctx = Context::new();

    ctx.insert("base_path", &state.path);
    let payload = render("dashboard", &state.tera, &ctx)?;
    Ok((ContentType::HTML, payload))
}

#[get("/login")]
pub async fn login(state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> {
    let mut ctx = Context::new();

    ctx.insert("base_path", &state.path);
    let payload = render("login", &state.tera, &ctx)?;
    Ok((ContentType::HTML, payload))
}

#[get("/view/<id>")]
pub async fn view_process(id: usize, state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> {
    let mut ctx = Context::new();

    ctx.insert("base_path", &state.path);
    ctx.insert("process_id", &id);

    let payload = render("view", &state.tera, &ctx)?;
    Ok((ContentType::HTML, payload))
}

#[get("/daemon/prometheus")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/prometheus", security((), ("api_key" = [])),
    responses(
        (
            description = "Get prometheus metrics", body = String, status = 200,
            example = json!("# HELP daemon_cpu_percentage The cpu usage graph of the daemon.\n# TYPE daemon_cpu_percentage histogram\ndaemon_cpu_percentage_bucket{le=\"0.005\"} 0"),
        ),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn prometheus_handler(_t: Token) -> String {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::<u8>::new();
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer.clone()).unwrap()
}

#[get("/daemon/servers")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/servers", security((), ("api_key" = [])),
    responses(
        (status = 200, description = "Get daemon servers successfully", body = [String]),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn servers_list(_t: Token) -> Result<Json<Vec<String>>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["servers"]).start_timer();

    if let Some(servers) = config::servers().servers {
        HTTP_COUNTER.inc();
        timer.observe_duration();

        Ok(Json(servers.into_keys().collect()))
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/daemon/server/<name>/list")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/server/{name}/list", security((), ("api_key" = [])),
    params(("name" = String, Path, description = "Name of remote daemon", example = "example"),),
    responses(
        (status = 200, description = "Get list from remote daemon successfully", body = [ProcessItem]),
        (status = NOT_FOUND, description = "Remote daemon does not exist", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn servers_handler(name: String, _t: Token) -> Result<Json<Vec<ProcessItem>>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["servers"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token)),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.get(fmtstr!("{address}/list")).headers(headers).send() {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<Vec<ProcessItem>>().unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/daemon/dump")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/dump", security((), ("api_key" = [])),
    responses(
        (status = 200, description = "Dump processes successfully", body = [u8]),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn dump_handler(_t: Token) -> Vec<u8> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["dump"]).start_timer();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    dump::raw()
}

#[get("/daemon/config")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/config", security((), ("api_key" = [])),
    responses(
        (status = 200, description = "Get daemon config successfully", body = ConfigBody),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn config_handler(_t: Token) -> Json<ConfigBody> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["dump"]).start_timer();
    let config = config::read().runner;

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Json(ConfigBody {
        shell: config.shell,
        args: config.args,
        log_path: config.log_path,
    })
}

#[get("/list")]
#[utoipa::path(get, path = "/list", tag = "Process", security((), ("api_key" = [])),
    responses(
        (status = 200, description = "List processes successfully", body = [ProcessItem]),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn list_handler(_t: Token) -> Json<Vec<ProcessItem>> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["list"]).start_timer();
    let data = Runner::new().fetch();

    HTTP_COUNTER.inc();
    timer.observe_duration();

    Json(data)
}

#[get("/process/<id>/logs/<kind>")]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/logs/{kind}", 
    security((), ("api_key" = [])),
    params(
        ("id" = usize, Path, description = "Process id to get logs for", example = 0),
        ("kind" = String, Path, description = "Log output type", example = "out")
    ),
    responses(
        (status = 200, description = "Process logs of {type} fetched", body = LogResponse),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn logs_handler(id: usize, kind: String, _t: Token) -> Result<Json<LogResponse>, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["log"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            let log_file = match kind.as_str() {
                "out" | "stdout" => item.logs().out,
                "error" | "stderr" => item.logs().error,
                _ => item.logs().out,
            };

            match File::open(log_file) {
                Ok(data) => {
                    let reader = BufReader::new(data);
                    let logs: Vec<String> = reader.lines().collect::<io::Result<_>>().unwrap();

                    timer.observe_duration();
                    Ok(Json(LogResponse { logs }))
                }
                Err(_) => Ok(Json(LogResponse { logs: vec![] })),
            }
        }
        None => {
            timer.observe_duration();
            Err(not_found("Process was not found"))
        }
    }
}

#[get("/process/<id>/logs/<kind>/raw")]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/logs/{kind}/raw", 
    security((), ("api_key" = [])),
    params(
        ("id" = usize, Path, description = "Process id to get logs for", example = 0),
        ("kind" = String, Path, description = "Log output type", example = "out")
    ),
    responses(
        (
            description = "Process logs of {type} fetched raw", body = String, status = 200,
            example = json!("# PATH path/of/file.log\nserver started on port 3000")
        ),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn logs_raw_handler(id: usize, kind: String, _t: Token) -> Result<String, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["log"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            let log_file = match kind.as_str() {
                "out" | "stdout" => item.logs().out,
                "error" | "stderr" => item.logs().error,
                _ => item.logs().out,
            };

            let data = match fs::read_to_string(&log_file) {
                Ok(data) => format!("# PATH {log_file}\n{data}"),
                Err(err) => err.to_string(),
            };

            timer.observe_duration();
            Ok(data)
        }
        None => {
            timer.observe_duration();
            Err(not_found("Process was not found"))
        }
    }
}

#[get("/process/<id>/info")]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/info", security((), ("api_key" = [])),
    params(("id" = usize, Path, description = "Process id to get information for", example = 0)),
    responses(
        (status = 200, description = "Current process info retrieved", body = ItemSingle),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn info_handler(id: usize, _t: Token) -> Result<Json<ItemSingle>, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["info"]).start_timer();
    let runner = Runner::new();

    if runner.exists(id) {
        let item = runner.get(id);
        HTTP_COUNTER.inc();
        timer.observe_duration();
        Ok(Json(item.fetch()))
    } else {
        Err(not_found("Process was not found"))
    }
}

#[post("/process/create", format = "json", data = "<body>")]
#[utoipa::path(post, tag = "Process", path = "/process/create", request_body(content = CreateBody), 
    security((), ("api_key" = [])),
    responses(
        (
            description = "Create process successful", body = ActionResponse,
            example = json!({"action": "create", "done": true }), status = 200,
        ),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to create process", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn create_handler(body: Json<CreateBody>, _t: Token) -> Result<Json<ActionResponse>, ()> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["create"]).start_timer();
    let mut runner = Runner::new();

    HTTP_COUNTER.inc();

    let name = match &body.name {
        Some(name) => string!(name),
        None => string!(body.script.split_whitespace().next().unwrap_or_default()),
    };

    runner.start(&name, &body.script, body.path.clone(), &body.watch).save();
    timer.observe_duration();

    Ok(Json(attempt(true, "create")))
}

#[post("/process/<id>/rename", format = "text", data = "<body>")]
#[utoipa::path(post, tag = "Process", path = "/process/{id}/rename", 
    security((), ("api_key" = [])),
    request_body(content = String, example = json!("example_name")), 
    params(("id" = usize, Path, description = "Process id to rename", example = 0)),
    responses(
        (
            description = "Rename process successful", body = ActionResponse,
            example = json!({"action": "rename", "done": true }), status = 200,
        ),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn rename_handler(id: usize, body: String, _t: Token) -> Result<Json<ActionResponse>, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["rename"]).start_timer();
    let runner = Runner::new();

    match runner.clone().info(id) {
        Some(process) => {
            HTTP_COUNTER.inc();
            let mut item = runner.get(id);
            item.rename(body.trim().replace("\n", ""));
            then!(process.running, item.restart());
            timer.observe_duration();
            Ok(Json(attempt(true, "rename")))
        }
        None => {
            timer.observe_duration();
            Err(not_found("Process was not found"))
        }
    }
}

#[get("/process/<id>/env")]
#[utoipa::path(get, tag = "Process", path = "/process/{id}/env",
    params(("id" = usize, Path, description = "Process id to fetch env from", example = 0)),
    responses(
        (
            description = "Current process env", body = HashMap<String, String>,
            example = json!({"ENV_TEST_VALUE": "example_value"}), status = 200
        ),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn env_handler(id: usize, _t: Token) -> Result<EnvList, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["env"]).start_timer();

    HTTP_COUNTER.inc();
    match Runner::new().info(id) {
        Some(item) => {
            timer.observe_duration();
            Ok(Json(item.clone().env))
        }
        None => {
            timer.observe_duration();
            Err(not_found("Process was not found"))
        }
    }
}

#[post("/process/<id>/action", format = "json", data = "<body>")]
#[utoipa::path(post, tag = "Process", path = "/process/{id}/action", request_body = ActionBody,
    security((), ("api_key" = [])),
    params(("id" = usize, Path, description = "Process id to run action on", example = 0)),
    responses(
        (status = 200, description = "Run action on process successful", body = ActionResponse),
        (status = NOT_FOUND, description = "Process/action was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn action_handler(id: usize, body: Json<ActionBody>, _t: Token) -> Result<Json<ActionResponse>, NotFound> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["action"]).start_timer();
    let mut runner = Runner::new();
    let method = body.method.as_str();

    if runner.exists(id) {
        HTTP_COUNTER.inc();
        match method {
            "start" | "restart" => {
                runner.get(id).restart();
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            "stop" | "kill" => {
                runner.get(id).stop();
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            "remove" | "delete" => {
                runner.remove(id);
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            _ => {
                timer.observe_duration();
                Err(not_found("Process was not found"))
            }
        }
    } else {
        Err(not_found("Process was not found"))
    }
}

#[get("/daemon/metrics")]
#[utoipa::path(get, tag = "Daemon", path = "/daemon/metrics", security((), ("api_key" = [])),
    responses(
        (status = 200, description = "Get daemon metrics", body = MetricsRoot),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn metrics_handler(_t: Token) -> Json<MetricsRoot> {
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

    let memory_usage = match memory_usage {
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

    timer.observe_duration();
    Json(MetricsRoot {
        version: Version {
            pkg: format!("v{}", env!("CARGO_PKG_VERSION")),
            hash: env!("GIT_HASH_FULL"),
            build_date: env!("BUILD_DATE"),
            target: env!("PROFILE"),
        },
        daemon: Daemon {
            pid,
            uptime,
            running: pid::exists(),
            process_count: runner.count(),
            daemon_type: global!("pmc.daemon.kind"),
            stats: Stats { memory_usage, cpu_percent },
        },
    })
}
