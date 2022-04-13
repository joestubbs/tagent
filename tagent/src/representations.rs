use crate::db::DbPool;

use super::models::DbAcl;
use actix_web::{HttpResponse, ResponseError};
// use glob;
use jwt_simple::algorithms::RS256PublicKey;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};

pub struct AppState {
    pub app_version: String,
    pub root_dir: PathBuf,
    pub pub_key: RS256PublicKey,
    pub db_pool: DbPool,
}

// Ready Endpoint ----------

#[derive(Serialize)]
pub struct Ready {
    pub message: String,
    pub status: String,
    pub result: String,
    pub version: String,
}

// Error Structs ----------

/// An error type representing the errors that can be generated during auth checks.
#[derive(Debug)]
pub enum AuthCheckError {
    Db(diesel::result::Error),
    Glb(glob::PatternError),
}

impl fmt::Display for AuthCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<diesel::result::Error> for AuthCheckError {
    fn from(error: diesel::result::Error) -> Self {
        AuthCheckError::Db(error)
    }
}

impl From<glob::PatternError> for AuthCheckError {
    fn from(error: glob::PatternError) -> Self {
        AuthCheckError::Glb(error)
    }
}

// The basic representation of an HTTP Error response
#[derive(Serialize)]
pub struct ErrorRsp {
    pub message: String,
    pub status: String,
    pub result: String,
    pub version: String,
}

/// Primary error type used by hanlders. It can convert to a actix_web::ResponseError
#[derive(Debug, PartialEq)]
pub struct TagentError {
    message: String,
    version: String,
}

// constructors -----
impl TagentError {
    pub fn new(message: String, version: String) -> Self {
        TagentError { message, version }
    }

    /// Construct a TagentError with the version set automatically from the cargo package version.
    pub fn new_with_version(message: String) -> Self {
        Self::new(message, String::from(env!("CARGO_PKG_VERSION")))
    }
}

// From impls for error conversion -----
impl From<&str> for TagentError {
    fn from(message: &str) -> Self {
        TagentError::new_with_version(String::from(message))
    }
}

impl From<String> for TagentError {
    fn from(message: String) -> Self {
        TagentError::new_with_version(message)
    }
}

impl From<diesel::result::Error> for TagentError {
    fn from(e: diesel::result::Error) -> Self {
        TagentError::new_with_version(format!("Error accessing database: {}", e))
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

impl From<glob::PatternError> for TagentError {
    /// Converts a glob::PatternError which can be generated when a user-provided ACL path is an invalid glob.
    fn from(error: glob::PatternError) -> Self {
        TagentError::new_with_version(format!("Invalid glob path; details: {}", error))
    }
}

impl From<std::io::Error> for TagentError {
    fn from(error: std::io::Error) -> Self {
        TagentError::new_with_version(format!("IO Error: {}", error))
    }
}

impl From<reqwest::Error> for TagentError {
    fn from(error: reqwest::Error) -> Self {
        TagentError::new_with_version(format!("Request Error: {}", error))
    }
}

impl From<AuthCheckError> for TagentError {
    fn from(error: AuthCheckError) -> Self {
        TagentError::new_with_version(format!(
            "Unable to calculate auth check; details: {}",
            error
        ))
    }
}

use r2d2::Error;
impl From<Error> for TagentError {
    fn from(error: Error) -> Self {
        TagentError::new_with_version(format!(
            "Unable to get database connection; details: {}",
            error
        ))
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

// ACL Endpoints ----------

/// Respones for ACL endpoints that return a string result
#[derive(Serialize, Deserialize)]
pub struct AclStringRsp {
    pub message: String,
    pub status: String,
    pub result: String,
    pub version: String,
}

/// A representation of an ACL that can be used in JSON responses that contain an ACL result or a Vector of ACLs
#[derive(Debug, Serialize, Deserialize)]
pub struct Acl {
    pub id: i32,
    pub subject: String,
    pub action: String,
    pub path: String,
    pub user: String,
    pub decision: String,
    pub create_by: String,
    pub create_time: String,
}

impl Acl {
    /// Convert a database representation of an ACL (Dbacl) to an Acl formatted for a response.
    pub fn from_db_acl(db_acl: &DbAcl) -> Self {
        Acl {
            id: db_acl.id,
            subject: db_acl.subject.clone(),
            action: db_acl.action.clone(),
            path: db_acl.path.clone(),
            user: db_acl.user.clone(),
            decision: db_acl.decision.clone(),
            create_by: db_acl.create_by.clone(),
            create_time: db_acl.create_time.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AclListingRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: Vec<Acl>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AclByIdRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: Acl,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthAnswer {
    pub allowed: bool,
    pub acl_id: Option<i32>,
}

/// Respones for ACL endpoints that return a string result
#[derive(Serialize, Deserialize)]
pub struct AclAuthzCheckRsp {
    pub message: String,
    pub status: String,
    pub result: AuthAnswer,
    pub version: String,
}

// Files Endpoints ----------

#[derive(Serialize, Deserialize)]
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
