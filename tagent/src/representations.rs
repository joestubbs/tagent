use actix_web::{HttpResponse, ResponseError};
use jwt_simple::algorithms::RS256PublicKey;
use serde::Serialize;
use std::fmt;
use super::models::DbAcl;

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
pub struct CreateAclRsp {
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
pub struct Acl {
    pub subject: String,
    pub action: String,
    pub path: String,
    pub user: String,
    pub create_by: String,
    pub create_time: String,
}

impl Acl {
    pub fn from_db_acl(db_acl: &DbAcl) -> Self {
        Acl{
            subject: db_acl.subject.clone(),
            action: db_acl.action.clone(),
            path: db_acl.path.clone(),
            user: db_acl.user.clone(),
            create_by: db_acl.create_by.clone(),
            create_time: db_acl.create_time.clone(),
        }

    }
    
}

#[derive(Debug, Serialize)]
pub struct AclListingRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: Vec<Acl>,
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
