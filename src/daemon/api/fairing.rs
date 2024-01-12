use rocket::fairing::{Fairing, Info, Kind};
use rocket::{http::ContentType, Data, Orbit, Request, Response, Rocket};

#[rocket::async_trait]
impl Fairing for super::Logger {
    fn info(&self) -> Info {
        Info {
            name: "Logger Fairing",
            kind: Kind::Liftoff | Kind::Request | Kind::Response,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let config = rocket.config();

        log!("[rocket] launched",
            "tls" => config.tls_enabled(),
            "keep_alive" => config.keep_alive,
            "workers" => config.workers,
            "profile" => config.profile.to_string(),
        );

        log!("[rocket] limits", "limits" => config.limits);
        log!("[api] server started", "port" => config.port, "host" => config.address);
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        log!("[api] req",
           "method" => request.method(),
           "uri" => request.uri(),
           "content_type" => request.content_type().unwrap_or(&ContentType::Plain),
        );
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        log!("[api] res",
           "status" => response.status(),
           "size" => response.body_mut().size().await.unwrap_or(0),
           "content_type" => response.content_type().unwrap_or(ContentType::Plain),
        );
    }
}
