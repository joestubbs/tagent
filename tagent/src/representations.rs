use actix_web::{HttpResponse, ResponseError};
use jwt_simple::algorithms::RS256PublicKey;
use serde::Serialize;
use std::{fmt, path::PathBuf};

pub struct AppState {
    pub app_version: String,
    pub root_dir: PathBuf,
    pub pub_key: RS256PublicKey,
}

#[derive(Serialize)]
pub struct Ready {
    pub message: String,
    pub status: String,
    pub result: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct ErrorRsp {
    pub message: String,
    pub status: String,
    pub result: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct FileListingRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: Vec<String>,
}

#[derive(Serialize)]
pub struct FileUploadRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: String,
}

// The Error type that can convert to a actix_web::HttpResponse
#[derive(Debug)]
pub struct TagentError {
    message: String,
    version: String,
}

impl TagentError {
    pub fn new(message: String, version: String) -> Self {
        TagentError { message, version }
    }

    pub fn new_with_version(message: String) -> Self {
        Self::new(message, String::from(env!("CARGO_PKG_VERSION")))
    }
}

impl From<&str> for TagentError {
    fn from(message: &str) -> Self {
        TagentError::new_with_version(String::from(message))
    }
}

impl From<TagentError> for std::io::Error {
    fn from(tagent_error: TagentError) -> Self {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "TagentError (version: {}): {}",
                tagent_error.message, tagent_error.version
            ),
        )
    }
}

impl fmt::Display for TagentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl ResponseError for TagentError {
    fn error_response(&self) -> HttpResponse {
        let m = &self.message;
        let v = &self.version;
        let r = ErrorRsp {
            status: String::from("error"),
            message: m.to_string(),
            version: v.to_string(),
            result: String::from("none"),
        };
        let body = serde_json::to_value(&r).unwrap().to_string();
        HttpResponse::BadRequest().body(body)
    }
}
