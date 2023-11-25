use macros_rs::fmtstr;
use pmc::{config, process::Runner};
use serde::Serialize;
use std::convert::Infallible;
use warp::{http::StatusCode, reject, reply::json, Filter, Rejection, Reply};

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

pub async fn start() {
    let config = config::read().daemon.api;

    let health = warp::path!("health").map(|| format!("ok!"));
    let list = warp::path!("list").and_then(list_handler);
    let info = warp::path!("info" / usize).and_then(|id| info_handler(id));
    let routes = warp::get().and(health.or(list).or(info)).recover(handle_rejection);

    if config.secure.enabled {
        let auth = warp::header::exact("authorization", fmtstr!("token {}", config.secure.token));
        warp::serve(routes.and(auth)).run(config::read().get_address()).await
    } else {
        warp::serve(routes).run(config::read().get_address()).await
    }
}

#[inline]
async fn list_handler() -> Result<impl Reply, Infallible> { Ok(json(&Runner::new().json())) }

#[inline]
async fn info_handler(id: usize) -> Result<impl Reply, Rejection> {
    match Runner::new().info(id) {
        Some(item) => Ok(json(&item.clone().json())),
        None => Err(reject::not_found()),
    }
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        log!("(API) unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
