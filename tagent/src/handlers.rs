use actix_files::NamedFile;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder, Result};
use log::{debug, info};
use std::fs;
use std::path::{Path, PathBuf};

use actix_multipart::Multipart;
use async_std::prelude::*;
use futures::{StreamExt, TryStreamExt};

use uuid::Uuid;

use crate::models::AclAction;

use super::auth::get_subject_of_request;
use super::db::{
    delete_acl_from_db_by_id, establish_connection, is_authz_db, retrieve_acl_by_id,
    retrieve_acls_for_subject, retrieve_acls_for_subject_user, retrieve_all_acls, save_acl,
    update_acl_in_db_by_id,
};
use super::models::NewAclJson;
use super::representations::{
    Acl, AclByIdRsp, AclListingRsp, AclStringRsp, AppState, FileListingRsp, FileUploadRsp, Ready,
    TagentError,
};

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
pub async fn create_acl(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    acl: web::Json<NewAclJson>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;
    debug!("processing request to POST /acls");
    let subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let _r = save_acl(
        &mut conn,
        &acl.subject,
        &acl.action,
        &acl.path,
        &acl.user,
        &acl.decision,
        &subject,
    )?;
    let rsp = AclStringRsp {
        status: String::from("success"),
        message: format!("ACL for subject {} created successfully.", acl.subject),
        result: String::from("none"),
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls")]
pub async fn get_all_acls(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;
    debug!("processing request to GET /acls/all");
    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let acls_db = retrieve_all_acls(&mut conn)?;
    let mut acls = Vec::<Acl>::new();
    for a in &acls_db {
        acls.push(Acl::from_db_acl(a));
    }

    let rsp = AclListingRsp {
        status: String::from("success"),
        message: "ACLs retrieved successfully.".to_string(),
        result: acls,
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls/{id}")]
pub async fn get_acl_by_id(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let id = path.0;

    debug!("processing request to GET /acls/{}", id);
    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let result = retrieve_acl_by_id(&mut conn, id)?;

    let acl = Acl::from_db_acl(&result);

    let rsp = AclByIdRsp {
        status: String::from("success"),
        message: "ACL retrieved successfully.".to_string(),
        result: acl,
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[delete("/acls/{id}")]
pub async fn delete_acl_by_id(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(i32,)>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let acl_id = path.0;

    debug!("processing request to DELETE /acls/{}", acl_id);
    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let _result = delete_acl_from_db_by_id(&mut conn, acl_id)?;
    
    let rsp = AclStringRsp {
        status: String::from("success"),
        message: "ACL deleted successfully.".to_string(),
        result: "none".to_string(),
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[put("/acls/{id}")]
pub async fn update_acl_by_id(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(i32,)>,
    acl: web::Json<NewAclJson>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let acl_id = path.0;

    debug!("processing request to PUT /acls/{}", acl_id);
    let subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let _result = update_acl_in_db_by_id(&mut conn, acl_id, &acl, &subject)?;

    let rsp = AclStringRsp {
        status: String::from("success"),
        message: "ACL updated successfully.".to_string(),
        result: "none".to_string(),
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls/subject/{subject}")]
pub async fn get_acls_for_subject(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(String,)>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let sub = &path.0;

    debug!("processing request to GET /acls/subject/{}", sub);
    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let results = retrieve_acls_for_subject(&mut conn, sub)?;
    let mut acls = Vec::<Acl>::new();
    for a in &results {
        acls.push(Acl::from_db_acl(a));
    }

    let rsp = AclListingRsp {
        status: String::from("success"),
        message: "ACLs retrieved successfully.".to_string(),
        result: acls,
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls/subject/{subject}/{user}")]
pub async fn get_acls_for_subject_user(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let sub = &path.0;
    let user = &path.1;

    debug!("processing request to GET /acls/subject/{}/{}", sub, user);
    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let results = retrieve_acls_for_subject_user(&mut conn, sub, user)?;

    let mut acls = Vec::<Acl>::new();
    for a in &results {
        acls.push(Acl::from_db_acl(a));
    }

    let rsp = AclListingRsp {
        status: String::from("success"),
        message: "ACLs retrieved successfully.".to_string(),
        result: acls,
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

#[get("/acls/isauthz/{subject}/{user}/{action}/{path:.*}")]
pub async fn is_authz_subject_user_action_path(
    _req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<(String, String, AclAction, String)>,
) -> Result<impl Responder, TagentError> {
    let version = &app_state.get_ref().app_version;
    let pub_key = &app_state.get_ref().pub_key;

    let sub = &path.0;
    let usr = &path.1;
    let act = &path.2;
    let pth = &path.3;

    // all paths start with a slash
    let mut check_path = String::from('/');
    if !(pth.starts_with('/')) {
        check_path.push_str(&pth);
    } else {
        check_path = pth.to_string();
    }

    debug!(
        "processing request to GET /acls/isauthz/{}/{}/{}/{:#?}",
        sub, usr, act, check_path
    );

    let _subject = get_subject_of_request(_req, pub_key).await?;
    let mut conn = establish_connection();
    let result = is_authz_db(&mut conn, sub, usr, &check_path, act)?;

    let rsp = AclStringRsp {
        status: String::from("success"),
        message: "Result of authz check returned".to_string(),
        result: result.to_string(),
        version: version.to_string(),
    };

    Ok(web::Json(rsp))
}

// Utils
// TODO -- move these utils functions to a separate module?

// Returns None if the input is not valid UTF-8.
pub fn path_buf_to_str(input: &Path) -> Option<&str> {
    input.to_str()
}

// Returns None if the input is not valid UTF-8.
pub fn path_buf_to_string(input: PathBuf) -> Option<String> {
    input.as_path().to_str().map(|s| s.to_string())
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

// TODO -- remove type alias?
// TODO -- should these retuen Impl Responder like the ACL endpoints?
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
    // TODO -- specify PathBuf type in function signature?
    let path = params.0;
    debug!("processing request to GET /files/list/{}", path);
    let subject = get_subject_of_request(_req, pub_key).await?;
    debug!("parsed jwt; subject: {}", subject);

    let mut full_path = PathBuf::from(root_dir);
    if path != "/" {
        full_path.push(path);
    }
    if !full_path.exists() {
        let message = format!(
            "Invalid path; path {:#?} does not exist",
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

// TODO -- remove?
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
        message = format!("Invalid path; path {:#?} does not exist", &full_path);
        error = true;
    };
    if full_path.is_dir() {
        message = String::from("Directory download is not supported");
        error = true;
    };
    if error {
        return Err(TagentError::new(message, version.to_string()));
    }

    let fbody = NamedFile::open(full_path)?;
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

// TODO -- remove?
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
            "Invalid path; path {:#?} does not exist",
            path_buf_to_str(&full_path)
        );
        error = true;
    };
    if !full_path.is_dir() {
        message = format!("Invalid path; path {:#?} must be a directory", full_path);
        error = true;
    };
    if error {
        return Err(TagentError::new(message, version.to_string()));
    };
    let full_path_s = path_buf_to_string(full_path).unwrap();
    let upload_path = save_file(payload, &full_path_s).await?;
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

    #[actix_rt::test]
    async fn status_should_be_ready() -> std::io::Result<()> {
        let pub_str = String::from("-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtsQsUV8QpqrygsY+2+JCQ6Fw8/omM71IM2N/R8pPbzbgOl0p78MZ\nGsgPOQ2HSznjD0FPzsH8oO2B5Uftws04LHb2HJAYlz25+lN5cqfHAfa3fgmC38Ff\nwBkn7l582UtPWZ/wcBOnyCgb3yLcvJrXyrt8QxHJgvWO23ITrUVYszImbXQ67YGS\n0YhMrbixRzmo2tpm3JcIBtnHrEUMsT0NfFdfsZhTT8YbxBvA8FdODgEwx7u/vf3J\n9qbi4+Kv8cvqyJuleIRSjVXPsIMnoejIn04APPKIjpMyQdnWlby7rNyQtE4+CV+j\ncFjqJbE/Xilcvqxt6DirjFCvYeKYl1uHLwIDAQAB\n-----END RSA PUBLIC KEY-----");
        let app_state = AppState {
            app_version: String::from("0.1.0"),
            root_dir: PathBuf::from(""),
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
