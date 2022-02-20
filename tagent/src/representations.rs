use actix_web::{HttpResponse, ResponseError};
use jwt_simple::algorithms::RS256PublicKey;
use serde::Serialize;
use std::fmt;

pub struct AppState {
    pub app_version: String,
    pub root_dir: String,
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

#[derive(Debug, Serialize)]
pub struct Acls {
    pub subject: String,
    pub action: String,
    pub path: String,
    pub user: String,
    pub create_by: String,
    pub create_time: String,
}

#[derive(Debug, Serialize)]
pub struct AclListingRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: Vec<Acls>,
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

pub fn make_tagent_error(message: String, version: String) -> TagentError {
    TagentError { message, version }
}
