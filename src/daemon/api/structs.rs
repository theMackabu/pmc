use rocket::serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub(crate) struct ErrorMessage {
    #[schema(example = 404)]
    pub(crate) code: u16,
    #[schema(example = "NOT_FOUND")]
    pub(crate) message: &'static str,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AuthMessage {
    #[schema(example = 401)]
    pub(crate) code: u16,
    #[schema(example = "UNAUTHORIZED")]
    pub(crate) message: String,
}
