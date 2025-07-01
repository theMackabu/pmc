#![allow(non_snake_case)]

use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{fmtstr, string, ternary, then};
use prometheus::{Encoder, TextEncoder};
use psutil::process::Process;
use reqwest::header::HeaderValue;
use tera::Context;
use utoipa::ToSchema;

use rocket::{
    get,
    http::{ContentType, Status},
    post,
    response::stream::{Event, EventStream},
    serde::{json::Json, Deserialize, Serialize},
    State,
};

use super::{
    helpers::{generic_error, not_found, GenericError, NotFound},
    render,
    structs::ErrorMessage,
    EnableWebUI, TeraState,
};

use pmc::{
    config, file, helpers,
    process::{dump, http::client, ItemSingle, ProcessItem, Runner, get_process_cpu_usage_percentage},
};

use crate::daemon::{
    api::{HTTP_COUNTER, HTTP_REQ_HISTOGRAM},
    pid::{self, Pid},
};

use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::PathBuf,
    thread::sleep,
    time::Duration,
};

pub(crate) struct Token;
type EnvList = Json<BTreeMap<String, String>>;

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

#[derive(Serialize, Deserialize, ToSchema)]
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

#[derive(Serialize, Deserialize, ToSchema)]
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

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ActionResponse {
    #[schema(example = true)]
    done: bool,
    #[schema(example = "name")]
    action: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct LogResponse {
    logs: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MetricsRoot {
    pub raw: Raw,
    pub version: Version,
    pub os: crate::globals::Os,
    pub daemon: Daemon,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Raw {
    pub memory_usage: Option<u64>,
    pub cpu_percent: Option<f64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Version {
    #[schema(example = "v1.0.0")]
    pub pkg: String,
    pub hash: Option<String>,
    #[schema(example = "2000-01-01")]
    pub build_date: String,
    #[schema(example = "release")]
    pub target: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Daemon {
    pub pid: Option<Pid>,
    #[schema(example = true)]
    pub running: bool,
    pub uptime: String,
    pub process_count: usize,
    #[schema(example = "default")]
    pub daemon_type: String,
    pub stats: Stats,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Stats {
    pub memory_usage: String,
    pub cpu_percent: String,
}

fn attempt(done: bool, method: &str) -> ActionResponse {
    ActionResponse {
        done,
        action: ternary!(done, Box::leak(Box::from(method)), "DOES_NOT_EXIST").to_string(),
    }
}

#[get("/")]
pub async fn dashboard(state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> { Ok((ContentType::HTML, render("dashboard", &state, &mut Context::new()).await?)) }

#[get("/servers")]
pub async fn servers(state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> { Ok((ContentType::HTML, render("servers", &state, &mut Context::new()).await?)) }

#[get("/login")]
pub async fn login(state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> { Ok((ContentType::HTML, render("login", &state, &mut Context::new()).await?)) }

#[get("/view/<id>")]
pub async fn view_process(id: usize, state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> {
    let mut ctx = Context::new();
    ctx.insert("process_id", &id);
    Ok((ContentType::HTML, render("view", &state, &mut ctx).await?))
}

#[get("/status/<name>")]
pub async fn server_status(name: String, state: &State<TeraState>, _webui: EnableWebUI) -> Result<(ContentType, String), NotFound> {
    let mut ctx = Context::new();
    ctx.insert("server_name", &name);
    Ok((ContentType::HTML, render("status", &state, &mut ctx).await?))
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
pub async fn servers_handler(_t: Token) -> Result<Json<Vec<String>>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["servers"]).start_timer();

    if let Some(servers) = config::servers().servers {
        HTTP_COUNTER.inc();
        timer.observe_duration();

        Ok(Json(servers.into_keys().collect()))
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/remote/<name>/list")]
#[utoipa::path(get, tag = "Remote", path = "/remote/{name}/list", security((), ("api_key" = [])),
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
pub async fn remote_list(name: String, _t: Token) -> Result<Json<Vec<ProcessItem>>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["list"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.get(fmtstr!("{address}/list")).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<Vec<ProcessItem>>().await.unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/remote/<name>/info/<id>")]
#[utoipa::path(get, tag = "Remote", path = "/remote/{name}/info/{id}", security((), ("api_key" = [])),
    params(
        ("name" = String, Path, description = "Name of remote daemon", example = "example"),
        ("id" = usize, Path, description = "Process id to get information for", example = 0)
    ),
    responses(
        (status = 200, description = "Get process info from remote daemon successfully", body = [ProcessItem]),
        (status = NOT_FOUND, description = "Remote daemon does not exist", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn remote_info(name: String, id: usize, _t: Token) -> Result<Json<ItemSingle>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["info"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.get(fmtstr!("{address}/process/{id}/info")).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<ItemSingle>().await.unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/remote/<name>/logs/<id>/<kind>")]
#[utoipa::path(get, tag = "Remote", path = "/remote/{name}/logs/{id}/{kind}", security((), ("api_key" = [])),
    params(
        ("name" = String, Path, description = "Name of remote daemon", example = "example"),
        ("id" = usize, Path, description = "Process id to get information for", example = 0),
        ("kind" = String, Path, description = "Log output type", example = "out")
    ),
    responses(
        (status = 200, description = "Remote process logs of {type} fetched", body = LogResponse),
        (status = NOT_FOUND, description = "Remote daemon does not exist", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn remote_logs(name: String, id: usize, kind: String, _t: Token) -> Result<Json<LogResponse>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["info"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.get(fmtstr!("{address}/process/{id}/logs/{kind}")).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<LogResponse>().await.unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[post("/remote/<name>/rename/<id>", format = "text", data = "<body>")]
#[utoipa::path(post, tag = "Remote", path = "/remote/{name}/rename/{id}", 
    security((), ("api_key" = [])),
    request_body(content = String, example = json!("example_name")), 
    params(
        ("id" = usize, Path, description = "Process id to rename", example = 0),
        ("name" = String, Path, description = "Name of remote daemon", example = "example"),
    ),
    responses(
        (
            description = "Remote rename process successful", body = ActionResponse,
            example = json!({"action": "rename", "done": true }), status = 200,
        ),
        (status = NOT_FOUND, description = "Process was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn remote_rename(name: String, id: usize, body: String, _t: Token) -> Result<Json<ActionResponse>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["rename"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, mut headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();
        headers.insert("content-type", HeaderValue::from_static("text/plain"));

        match client.post(fmtstr!("{address}/process/{id}/rename")).body(body).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<ActionResponse>().await.unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[post("/remote/<name>/action/<id>", format = "json", data = "<body>")]
#[utoipa::path(post, tag = "Remote", path = "/remote/{name}/action/{id}", request_body = ActionBody,
    security((), ("api_key" = [])),
    params(
        ("id" = usize, Path, description = "Process id to run action on", example = 0),
        ("name" = String, Path, description = "Name of remote daemon", example = "example")
    ),
    responses(
        (status = 200, description = "Run action on remote process successful", body = ActionResponse),
        (status = NOT_FOUND, description = "Process/action was not found", body = ErrorMessage),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn remote_action(name: String, id: usize, body: Json<ActionBody>, _t: Token) -> Result<Json<ActionResponse>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["action"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.post(fmtstr!("{address}/process/{id}/action")).json(&body.0).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<ActionResponse>().await.unwrap()))
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
            "reset_env" | "clear_env" => {
                runner.get(id).clear_env();
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            "remove" | "delete" => {
                runner.remove(id);
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            "flush" | "clean" => {
                runner.flush(id);
                timer.observe_duration();
                Ok(Json(attempt(true, method)))
            }
            _ => {
                timer.observe_duration();
                Err(not_found("Invalid action attempt"))
            }
        }
    } else {
        Err(not_found("Process was not found"))
    }
}

pub async fn get_metrics() -> MetricsRoot {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["metrics"]).start_timer();
    let os_info = crate::globals::get_os_info();

    let mut pid: Option<Pid> = None;
    let mut cpu_percent: Option<f64> = None;
    let mut uptime: Option<DateTime<Utc>> = None;
    let mut memory_usage: Option<u64> = None;
    let mut runner: Runner = file::read_object(global!("pmc.dump"));

    HTTP_COUNTER.inc();
    if pid::exists() {
        if let Ok(process_id) = pid::read() {
            if let Ok(process) = Process::new(process_id.get()) {
                pid = Some(process_id);
                uptime = Some(pid::uptime().unwrap());
                memory_usage = Some(process.memory_info().unwrap().rss());
                cpu_percent = Some(get_process_cpu_usage_percentage(process_id.get::<i64>()));
            }
        }
    }

    let memory_usage_fmt = match memory_usage {
        Some(usage) => helpers::format_memory(usage),
        None => string!("0b"),
    };

    let cpu_percent_fmt = match cpu_percent {
        Some(percent) => format!("{:.2}%", percent),
        None => string!("0.00%"),
    };

    let uptime_fmt = match uptime {
        Some(uptime) => helpers::format_duration(uptime),
        None => string!("none"),
    };

    timer.observe_duration();
    MetricsRoot {
        os: os_info.clone(),
        raw: Raw { memory_usage, cpu_percent },
        version: Version {
            target: env!("PROFILE").into(),
            build_date: env!("BUILD_DATE").into(),
            pkg: format!("v{}", env!("CARGO_PKG_VERSION")),
            hash: ternary!(env!("GIT_HASH_FULL") == "", None, Some(env!("GIT_HASH_FULL").into())),
        },
        daemon: Daemon {
            pid,
            uptime: uptime_fmt,
            running: pid::exists(),
            process_count: runner.count(),
            daemon_type: global!("pmc.daemon.kind"),
            stats: Stats {
                memory_usage: memory_usage_fmt,
                cpu_percent: cpu_percent_fmt,
            },
        },
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
pub async fn metrics_handler(_t: Token) -> Json<MetricsRoot> { Json(get_metrics().await) }

#[get("/remote/<name>/metrics")]
#[utoipa::path(get, tag = "Remote", path = "/remote/{name}/metrics", security((), ("api_key" = [])),
    params(("name" = String, Path, description = "Name of remote daemon", example = "example")),
    responses(
        (status = 200, description = "Get remote metrics", body = MetricsRoot),
        (
            status = UNAUTHORIZED, description = "Authentication failed or not provided", body = ErrorMessage, 
            example = json!({"code": 401, "message": "Unauthorized"})
        )
    )
)]
pub async fn remote_metrics(name: String, _t: Token) -> Result<Json<MetricsRoot>, GenericError> {
    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&["info"]).start_timer();

    if let Some(servers) = config::servers().servers {
        let (address, (client, headers)) = match servers.get(&name) {
            Some(server) => (&server.address, client(&server.token).await),
            None => return Err(generic_error(Status::NotFound, string!("Server was not found"))),
        };

        HTTP_COUNTER.inc();
        timer.observe_duration();

        match client.get(fmtstr!("{address}/daemon/metrics")).headers(headers).send().await {
            Ok(data) => {
                if data.status() != 200 {
                    let err = data.json::<ErrorMessage>().await.unwrap();
                    Err(generic_error(err.code, err.message))
                } else {
                    Ok(Json(data.json::<MetricsRoot>().await.unwrap()))
                }
            }
            Err(err) => Err(generic_error(Status::InternalServerError, err.to_string())),
        }
    } else {
        Err(generic_error(Status::BadRequest, string!("No servers have been added")))
    }
}

#[get("/live/daemon/<server>/metrics")]
pub async fn stream_metrics(server: String, _t: Token) -> EventStream![] {
    EventStream! {
        match config::servers().servers {
            Some(servers) => {
                let (address, (client, headers)) = match servers.get(&server) {
                    Some(server) => (&server.address, client(&server.token).await),
                    None => match &*server {
                        "local" | "internal" => loop {
                            let response = get_metrics().await;
                            yield Event::data(serde_json::to_string(&response).unwrap());
                            sleep(Duration::from_millis(500));
                        },
                        _ => return yield Event::data(format!("{{\"error\": \"server does not exist\"}}")),
                    }
                };

                loop {
                    match client.get(fmtstr!("{address}/daemon/metrics")).headers(headers.clone()).send().await {
                        Ok(data) => {
                            if data.status() != 200 {
                                break yield Event::data(data.text().await.unwrap());
                            } else {
                                yield Event::data(data.text().await.unwrap());
                                sleep(Duration::from_millis(1500));
                            }
                        }
                        Err(err) => break yield Event::data(format!("{{\"error\": \"{err}\"}}")),
                    }
                }
            }
            None => loop {
                let response = get_metrics().await;
                yield Event::data(serde_json::to_string(&response).unwrap());
                sleep(Duration::from_millis(500))
            },
        };
    }
}

#[get("/live/process/<server>/<id>")]
pub async fn stream_info(server: String, id: usize, _t: Token) -> EventStream![] {
    EventStream! {
        let runner = Runner::new();

        match config::servers().servers {
            Some(servers) => {
                let (address, (client, headers)) = match servers.get(&server) {
                    Some(server) => (&server.address, client(&server.token).await),
                    None => match &*server {
                        "local" | "internal" => loop {
                            let item = runner.refresh().get(id);
                            yield Event::data(serde_json::to_string(&item.fetch()).unwrap());
                            sleep(Duration::from_millis(1000));
                        },
                        _ => return yield Event::data(format!("{{\"error\": \"server does not exist\"}}")),
                    }
                };

                loop {
                    match client.get(fmtstr!("{address}/process/{id}/info")).headers(headers.clone()).send().await {
                        Ok(data) => {
                            if data.status() != 200 {
                                break yield Event::data(data.text().await.unwrap());
                            } else {
                                yield Event::data(data.text().await.unwrap());
                                sleep(Duration::from_millis(1500));
                            }
                        }
                        Err(err) => break yield Event::data(format!("{{\"error\": \"{err}\"}}")),
                    }
                }
            }
            None => loop {
                let item = runner.refresh().get(id);
                yield Event::data(serde_json::to_string(&item.fetch()).unwrap());
                sleep(Duration::from_millis(1000));
            }
        };
    }
}
