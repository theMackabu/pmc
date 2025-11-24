#![allow(dead_code)]

use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::json;

use utoipa::{
    ToSchema,
    openapi::{KnownFormat, Object, ObjectBuilder, SchemaFormat, SchemaType},
};

#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ErrorMessage {
    #[schema(schema_with = status)]
    pub(crate) code: Status,
    #[schema(example = "Not Found")]
    pub(crate) message: String,
}

fn status() -> Object {
    ObjectBuilder::new()
        .schema_type(SchemaType::Integer)
        .format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt16)))
        .example(Some(json!(404)))
        .build()
}
