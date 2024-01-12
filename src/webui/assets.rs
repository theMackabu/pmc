use rocket::{
    http::ContentType,
    request::Request,
    response::{self, Responder},
};

use std::{io, path::PathBuf};

#[derive(Debug)]
pub struct NamedFile(PathBuf, String);

impl NamedFile {
    pub async fn send(name: String, contents: Option<&str>) -> io::Result<NamedFile> { Ok(NamedFile(PathBuf::from(name), contents.unwrap().to_string())) }
}

impl<'r> Responder<'r, 'static> for NamedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let mut response = self.1.respond_to(req)?;
        if let Some(ext) = self.0.extension() {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }

        Ok(response)
    }
}
