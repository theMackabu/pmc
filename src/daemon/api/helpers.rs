use super::structs::ErrorMessage;
use rocket::{http::Status, response::status, serde::json::Json};

pub(crate) type NotFound = status::NotFound<Json<ErrorMessage>>;

pub(crate) fn create_status(status: Status) -> Json<ErrorMessage> {
    Json(ErrorMessage {
        code: status.code,
        message: status.reason_lossy(),
    })
}

pub(crate) fn not_found(msg: &'static str) -> NotFound {
    status::NotFound(Json(ErrorMessage {
        code: Status::NotFound.code,
        message: msg,
    }))
}
