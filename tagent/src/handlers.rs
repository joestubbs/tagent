use actix_files::NamedFile;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Result};
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

use actix_multipart::Multipart;
use async_std::prelude::*;
use futures::{StreamExt, TryStreamExt};

use uuid::Uuid;

use super::auth::get_subject_of_request;
use super::db::{establish_connection, save_acl, retrieve_all_acls};
use super::models::NewAclJson;
use super::representations::{AppState, FileListingRsp, FileUploadRsp, Ready, TagentError, CreateAclRsp, AclListingRsp, Acl};

// status endpoints ---
#[get("/status/ready")]
pub async fn ready(app_state: web::Data<AppState>) -> Result<impl Responder, TagentError> {
    debug!("processing request to GET /status/ready");
    let version = &app_state.get_ref().app_version;
    let r = Ready {
        status: String::from("success"),
        message: String::from("tagent ready."),
        result: String::from("None"),
        version: version.to_string(),
    };
    Ok(web::Json(r))
}

// acls endpoints ---
#[post("/acls")]
pub async fn create_acl(_req: HttpRequest, 
app_state: web::Data<AppState>, 
acl: web::Json<NewAclJson>) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;
    debug!("processing request to POST /acls");
    let subject = get_subject_of_request(_req, pub_key).await;
    let subject = match subject {
        Ok(sub) => sub,
        Err(error) => {
            let msg = format!("got an error from get_subject_of_request; error: {}", error);
            info!("{}", msg);
            return Err(TagentError::new(msg, version.to_string()));
        }
    };
    let mut conn = establish_connection();
    let r = save_acl(&mut conn, &acl.subject, &acl.action, &acl.path, &acl.user, &subject);
    let _r = match r {
        Ok(r) => r,
        Err(r) => return Err(TagentError::new(format!("Could not save ACL to db; details {}", r), version.to_string()))
    };
    let rsp = CreateAclRsp {
        status: String::from("success"),
        message: format!("ACL for subject {} created successfully.", acl.subject),
        result: String::from("none"),
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls/all")]
pub async fn get_all_acls(_req: HttpRequest, 
    app_state: web::Data<AppState>) 
    -> Result<impl Responder, TagentError> {

    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;
    debug!("processing request to POST /acls");
    let subject = get_subject_of_request(_req, pub_key).await;
    let _subject = match subject {
        Ok(sub) => sub,
        Err(error) => {
            let msg = format!("got an error from get_subject_of_request; error: {}", error);
            info!("{}", msg);
            return Err(TagentError::new(msg, version.to_string()));
        }
    };
    let mut conn = establish_connection();
    let acls_db = retrieve_all_acls(&mut conn);
    let acls_db = match acls_db {
        Ok(acls) => acls,
        Err(e) => return Err(TagentError::new(format!("Could not retrieve ACLs from db; details {}", e), version.to_string()))
    };
    
    let mut acls = Vec::<Acl>::new();
    for a in &acls_db {
        acls.push(Acl::from_db_acl(a));
    };

    let rsp = AclListingRsp {
        status: String::from("success"),
        message: format!("ACLs retrieved successfully."),
        result: acls,
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
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
        .or_else( |_| {warn!("TAGENT_HOME not set; using current working directory!"); std::env::current_dir()}
            .and_then(std::fs::canonicalize)
            .and_then(|x| path_to_string(&x)))
        .or_else(|_| {warn!("Couldn't use the curent working directory; defaulting to $HOME!"); std::env::var("HOME")} )
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

type FileListHttpRsp = Result<web::Json<FileListingRsp>, TagentError>;

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
    let subject = get_subject_of_request(_req, pub_key).await;
    let subject = match subject {
        Ok(sub) => sub,
        Err(error) => {
            let msg = format!("got an error from get_subject_of_request; error: {}", error);
            info!("{}", msg);
            return Err(TagentError::new(msg, version.to_string()));
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
        return Err(TagentError::new(message, version.to_string()));
    }
    let result = get_local_listing(full_path);

    let r = FileListingRsp {
        status: String::from("success"),
        message: String::from("File listing retrieved successfully"),
        version: version.to_string(),
        result,
    };
    Ok(web::Json(r))
}

// type FileContentsHttpRsp = Either<HttpResponse, Result<NamedFile>>;
type FileContentsHttpRsp = Result<HttpResponse, TagentError>;

#[get("/files/contents/{path:.*}")]
pub async fn get_file_contents_path(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    params: web::Path<(PathBuf,)>,
) -> FileContentsHttpRsp {
    let version = &app_state.get_ref().app_version;
    let root_dir = &app_state.get_ref().root_dir;
    let params = params.into_inner();
    let path = params.0;
    let mut full_path = PathBuf::from(root_dir);
    let mut error: bool = false;
    let mut message = String::from("There was an error");
    full_path.push(path);
    if !full_path.exists() {
        message = format!("Invalid path; path {:?} does not exist", &full_path);
        error = true;
    };
    if full_path.is_dir() {
        message = String::from("Directory download is not supported");
        error = true;
    };
    if error {
        return Err(TagentError::new(message, version.to_string()));
    }
    //this line compiles but doesn't allow for a custom error
    let fbody = NamedFile::open(full_path);
    let fbody = match fbody {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("Got error trying to open file; details: {}", e);
            return Err(TagentError::new(msg, version.to_string()));
        }
    };
    let res = fbody.into_response(&_req);
    Ok(res)
}

pub async fn save_file(mut payload: Multipart, full_path: &str) -> std::io::Result<String> {
    // cf., https://github.com/actix/examples/blob/master/forms/multipart/src/main.rs#L8
    // iterate over multipart stream
    let mut filepath = "na".to_string();
    while let Ok(Some(mut field)) = payload.try_next().await {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);

        filepath = format!("{}/{}", full_path, filename);

        let mut f = async_std::fs::File::create(&filepath).await?;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data =
                chunk.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            f.write_all(&data).await?;
        }
    }
    Ok(filepath)
}

type FileUploadHttpRsp = Result<web::Json<FileUploadRsp>, TagentError>;

#[post("/files/contents/{path:.*}")]
pub async fn post_file_contents_path(
    app_state: web::Data<AppState>,
    params: web::Path<(String,)>,
    payload: Multipart,
) -> FileUploadHttpRsp {
    let version = &app_state.get_ref().app_version;
    let root_dir = &app_state.get_ref().root_dir;
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
        return Err(TagentError::new(message, version.to_string()));
    };
    let full_path_s = path_buf_to_string(full_path).unwrap();
    let upload_path = save_file(payload, &full_path_s).await;
    let upload_path = match upload_path {
        Err(e) => {
            let message = format!("Unable to save file to disk; details: {}", e);
            return Err(TagentError::new(message, version.to_string()));
        }
        Ok(p) => p,
    };

    let r = FileUploadRsp {
        status: String::from("success"),
        message: format!("file uploaded to {} successfully.", upload_path),
        result: String::from("none"),
        version: version.to_string(),
    };

    Ok(web::Json(r))
}

#[cfg(test)]
mod test {
    use actix_web::App;
    use jwt_simple::algorithms::RS256PublicKey;
    use reqwest::StatusCode;

    use crate::make_config;

    use super::*;

    #[test]
    fn path_to_string_should_work_on_ascii() -> std::io::Result<()> {
        let s = path_to_string(Path::new("/foo/bar"))?;
        assert_eq!(s, "/foo/bar");
        Ok(())
    }

    #[test]
    fn path_to_string_should_work_on_unicode() -> std::io::Result<()> {
        let s = path_to_string(Path::new("/\u{2122}foo/bar"))?;
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

    #[actix_rt::test]
    async fn status_should_be_ready() -> std::io::Result<()> {
        let pub_str = String::from("-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtsQsUV8QpqrygsY+2+JCQ6Fw8/omM71IM2N/R8pPbzbgOl0p78MZ\nGsgPOQ2HSznjD0FPzsH8oO2B5Uftws04LHb2HJAYlz25+lN5cqfHAfa3fgmC38Ff\nwBkn7l582UtPWZ/wcBOnyCgb3yLcvJrXyrt8QxHJgvWO23ITrUVYszImbXQ67YGS\n0YhMrbixRzmo2tpm3JcIBtnHrEUMsT0NfFdfsZhTT8YbxBvA8FdODgEwx7u/vf3J\n9qbi4+Kv8cvqyJuleIRSjVXPsIMnoejIn04APPKIjpMyQdnWlby7rNyQtE4+CV+j\ncFjqJbE/Xilcvqxt6DirjFCvYeKYl1uHLwIDAQAB\n-----END RSA PUBLIC KEY-----");
        let app_state = AppState {
            app_version: String::from("0.1.0"),
            root_dir: String::from(""),
            pub_key: RS256PublicKey::from_pem(&pub_str).unwrap(),
        };
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let req = actix_web::test::TestRequest::get()
            .uri("/status/ready")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }
}
