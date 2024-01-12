use rocket::serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub(crate) struct ErrorMessage {
    #[schema(example = 404)]
    pub(crate) code: u16,
    #[schema(example = "Not Found")]
    pub(crate) message: &'static str,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AuthMessage {
    #[schema(example = 401)]
    pub(crate) code: u16,
    #[schema(example = "Unauthorized")]
    pub(crate) message: String,
}
