use actix_files::NamedFile;
use actix_web::{
    get, post, web, Either, Error, HttpRequest, HttpResponse, Responder, ResponseError, Result,
};
use log::{debug, info};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use actix_multipart::Multipart;
use async_std::prelude::*;
use futures::{StreamExt, TryStreamExt};
use uuid::Uuid;

use super::auth::get_sub;
use super::representations::{AppState, ErrorRsp, FileListingRsp, FileUploadRsp, Ready};

// status endpoints ---
#[get("/status/ready")]
pub async fn ready(app_version: web::Data<String>) -> Result<impl Responder> {
    debug!("processing request to GET /status/ready");
    let version: String = app_version.get_ref().to_string();
    let r = Ready {
        status: String::from("success"),
        message: String::from("tagent ready."),
        result: String::from("None"),
        version,
    };
    Ok(web::Json(r))
}

// acls endpoints ---
#[get("/acls/all")]
pub async fn get_all_acls() -> impl Responder {
    "todo: get_all_acls".to_string()
}

#[get("/acls/{service}")]
pub async fn get_acls_for_service() -> impl Responder {
    "todo: get_acls_for_service".to_string()
}

#[get("/acls/{service}/{user}")]
pub async fn get_acls_for_service_user() -> impl Responder {
    "todo: get_acls_for_service_user".to_string()
}

#[get("/acls/isauthz/{service}/{user}/{path:.*}")]
pub async fn is_authz_service_user_path() -> impl Responder {
    "todo: is_authz_service_user_path".to_string()
}

// Utils

// Returns None if the input is not valid UTF-8.
pub fn path_buf_to_str(input: &Path) -> Option<&str> {
    input.to_str()
}

// Returns None if the input is not valid UTF-8.
pub fn path_buf_to_string(input: PathBuf) -> Option<String> {
    input.as_path().to_str().map(|s| s.to_string())
}

fn path_to_string(input: &Path) -> std::io::Result<String> {
    input.to_str().map(String::from).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Couldn't convert path to string")
    })
}

pub(crate) fn get_root_dir() -> std::io::Result<String> {
    std::env::var("TAGENT_HOME")
        .or_else( |_| std::env::current_dir()
            .and_then(std::fs::canonicalize)
            .and_then(|x| path_to_string(&x)))
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Couldn't determine base directory.\nHelp: set the environment variable TAGENT_HOME.",
            )
        })
}

// files endpoints ---

pub fn get_local_listing(full_path: PathBuf) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    // check if full_path is a directory
    if !full_path.is_dir() {
        // assume it is a single path and return it
        result.push(full_path.to_string_lossy().to_string());
        return result;
    }
    let paths = fs::read_dir(full_path).unwrap();
    for path in paths {
        let s = path.unwrap().file_name().into_string(); // should be safe because we checked that full_path existed
        result.push(s.unwrap());
    }
    result
}

type FileListHttpRsp = Either<HttpResponse, web::Json<FileListingRsp>>;

#[get("/files/list/{path:.*}")]
pub async fn list_files_path(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    params: web::Path<(String,)>,
) -> FileListHttpRsp {
    let version = &app_state.get_ref().app_version;
    let root_dir = &app_state.get_ref().root_dir;
    let pub_key = &app_state.get_ref().pub_key;
    let params = params.into_inner();
    let path = params.0;
    debug!("processing request to GET /files/list/{}", path);
    debug!(
        "version: {}; root_dir: {}'; pub: {}",
        version, root_dir, pub_key
    );
    let subject = get_sub(_req, pub_key.to_string()).await;
    let subject = match subject {
        Ok(sub) => sub,
        Err(error) => {
            let msg = format!("got an error from get_subject_of_request; error: {}", error);
            info!("{}", msg);
            let r = ErrorRsp {
                status: String::from("error"),
                message: msg,
                version: version.to_string(),
                result: String::from("none"),
            };
            return Either::Left(HttpResponse::BadRequest().json(r));
        }
    };
    info!("parsed jwt; subject: {}", subject);

    let mut full_path = PathBuf::from(root_dir);
    if path != "/" {
        full_path.push(path);
    }
    if !full_path.exists() {
        let message = format!(
            "Invalid path; path {:?} does not exist",
            path_buf_to_str(&full_path)
        );
        let r = ErrorRsp {
            status: String::from("error"),
            message,
            version: version.to_string(),
            result: String::from("none"),
        };
        return Either::Left(HttpResponse::BadRequest().json(r));
    }
    let result = get_local_listing(full_path);

    let r = FileListingRsp {
        status: String::from("success"),
        message: String::from("File listing retrieved successfully"),
        version: version.to_string(),
        result,
    };
    Either::Right(web::Json(r))
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

pub fn make_tagent_error(message: String, version: String) -> Result<(), TagentError> {
    let r = TagentError { message, version };
    Err(r)
}

// type FileContentsHttpRsp = Either<HttpResponse, Result<NamedFile>>;
type FileContentsHttpRsp = Result<HttpResponse, Error>;

#[get("/files/contents/{path:.*}")]
pub async fn get_file_contents_path(
    _req: HttpRequest,
    app_version: web::Data<String>,
    root_dir: web::Data<String>,
    params: web::Path<(String,)>,
) -> FileContentsHttpRsp {
    let version = app_version.get_ref().to_string();
    let root_dir = root_dir.get_ref().to_string();
    let params = params.into_inner();
    let path = params.0;
    let mut full_path = PathBuf::from(root_dir);
    let mut error: bool = false;
    let mut message = String::from("There was an error");
    full_path.push(path);
    if !full_path.exists() {
        message = format!(
            "Invalid path; path {:?} does not exist",
            path_buf_to_str(&full_path)
        );
        error = true;
    };
    if full_path.is_dir() {
        message = String::from("Directory download is not supported");
        error = true;
    };
    if error {
        make_tagent_error(message, version)?;
    }
    //this line compiles but doesn't allow for a custom error
    let fbody = NamedFile::open(full_path)?;
    // let fbody = match fbody {
    //     Ok(f) => f,
    //     Err(e) => {
    //         let msg = format!("Got error trying to open file; details: {}", e);
    //         let er = make_tagent_error(msg, version.to_string())?;
    //     },
    // };
    let res = fbody.into_response(&_req);
    Ok(res)
}

pub async fn save_file(mut payload: Multipart, full_path: &str) -> Option<String> {
    // cf., https://github.com/actix/examples/blob/master/forms/multipart/src/main.rs#L8
    let mut filepath = String::from("empty");
    // iterate over multipart stream
    while let Ok(Some(mut field)) = payload.try_next().await {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);

        filepath = format!("{}/{}", full_path, filename);

        let mut f = async_std::fs::File::create(&filepath).await.unwrap();

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f.write_all(&data).await.unwrap();
        }
    }
    Some(filepath)
}

type FileUploadHttpRsp = Either<HttpResponse, web::Json<FileUploadRsp>>;

#[post("/files/contents/{path:.*}")]
pub async fn post_file_contents_path(
    app_version: web::Data<String>,
    root_dir: web::Data<String>,
    params: web::Path<(String,)>,
    payload: Multipart,
) -> FileUploadHttpRsp {
    let version = app_version.get_ref().to_string();
    let root_dir = root_dir.get_ref().to_string();
    let params = params.into_inner();
    let path = params.0;
    let mut full_path = PathBuf::from(root_dir);
    let mut error: bool = false;
    let mut message = String::from("There was an error");
    full_path.push(path);
    if !full_path.exists() {
        message = format!(
            "Invalid path; path {:?} does not exist",
            path_buf_to_str(&full_path)
        );
        error = true;
    };
    if !full_path.is_dir() {
        message = format!("Invalid path; path {:?} must be a directory", full_path);
        error = true;
    };
    if error {
        let r = ErrorRsp {
            status: String::from("error"),
            message,
            version: version.to_string(),
            result: String::from("none"),
        };
        return Either::Left(HttpResponse::BadRequest().json(r));
    };
    let full_path_s = path_buf_to_string(full_path).unwrap();
    let upload_path = save_file(payload, &full_path_s).await;

    let r = FileUploadRsp {
        status: String::from("success"),
        message: format!("file uploaded to {:?} successfully.", upload_path),
        result: String::from("none"),
        version: version.to_string(),
    };

    Either::Right(web::Json(r))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn path_to_string_should_work_on_ascii() -> std::io::Result<()> {
        let s = path_to_string(&Path::new("/foo/bar"))?;
        assert_eq!(s, "/foo/bar");
        Ok(())
    }

    #[test]
    fn path_to_string_should_work_on_unicode() -> std::io::Result<()> {
        let s = path_to_string(&Path::new("/\u{2122}foo/bar"))?;
        assert_eq!(s, "/\u{2122}foo/bar");
        Ok(())
    }

    #[test]
    fn get_root_dir_with_tagent_home_var() -> std::io::Result<()> {
        std::env::set_var("TAGENT_HOME", "bar");
        let r = get_root_dir()?;
        assert_eq!(r, "bar");
        Ok(())
    }

    #[test]
    fn get_root_dir_with_current_dir() -> std::io::Result<()> {
        let temp = tempfile::TempDir::new()?;
        std::env::set_current_dir(&temp)?;
        std::env::remove_var("TAGENT_HOME");
        let r = get_root_dir()?;
        assert_eq!(r, std::fs::canonicalize(temp)?.to_str().unwrap());
        Ok(())
    }

    #[test]
    fn get_root_dir_with_user_home() -> std::io::Result<()> {
        std::env::remove_var("TAGENT_HOME");
        {
            let temp = tempfile::TempDir::new()?;
            std::env::set_current_dir(temp)?;
            // temp gets deleted when going out of scope, so
            // current_dir becomes invalid
        }
        std::env::set_var("HOME", "baz");
        let a = get_root_dir()?;
        assert_eq!(a, "baz");
        Ok(())
    }

    #[test]
    fn get_root_dir_should_fail_if_no_vars_or_current_dir() -> std::io::Result<()> {
        std::env::remove_var("TAGENT_HOME");
        {
            let temp = tempfile::TempDir::new()?;
            std::env::set_current_dir(temp)?;
            // temp gets deleted when going out of scope, so
            // current_dir becomes invalid
        }
        std::env::remove_var("HOME");
        let a = get_root_dir();
        assert!(a.is_err());
        Ok(())
    }
}
