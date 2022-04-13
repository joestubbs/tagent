use actix_files::NamedFile;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder, Result};
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

use actix_multipart::Multipart;
use async_std::prelude::*;
use futures::{StreamExt, TryStreamExt};

use uuid::Uuid;

use crate::models::AclAction;
use crate::representations::AclAuthzCheckRsp;

use super::auth::get_subject_of_request;
use super::db::{
    delete_acl_from_db_by_id, is_authz_db, retrieve_acl_by_id, retrieve_acls_for_subject,
    retrieve_acls_for_subject_user, retrieve_all_acls, save_acl, update_acl_in_db_by_id,
};
use super::models::NewAclJson;
use super::representations::{
    Acl, AclByIdRsp, AclListingRsp, AclStringRsp, AppState, FileListingRsp, FileUploadRsp, Ready,
    TagentError,
};

// default service -- called when no handler matches the request
pub async fn not_found() -> Result<HttpResponse, TagentError> {
    Err(TagentError::new_with_version(
        "The API endpoint does not exist; check the URL path and HTTP verb.".to_string(),
    ))
}

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
    let conn = &app_state.get_ref().db_pool.get()?;
    let _r = acl.validate_path()?;
    let _r = save_acl(
        conn,
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
    let conn = &app_state.get_ref().db_pool.get()?;
    let acls_db = retrieve_all_acls(conn)?;
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
    let conn = &app_state.get_ref().db_pool.get()?;
    let result = retrieve_acl_by_id(conn, id)?;

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
    // let is_authz = is_authz_wrapper(_req, ....)?;
    let conn = &app_state.get_ref().db_pool.get()?;
    let _result = delete_acl_from_db_by_id(conn, acl_id)?;

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
    let conn = &app_state.get_ref().db_pool.get()?;
    let _result = update_acl_in_db_by_id(conn, acl_id, &acl, &subject)?;

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
    let conn = &app_state.get_ref().db_pool.get()?;
    let results = retrieve_acls_for_subject(conn, sub)?;
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
    let conn = &app_state.get_ref().db_pool.get()?;
    let results = retrieve_acls_for_subject_user(conn, sub, user)?;

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
        check_path.push_str(pth);
    } else {
        check_path = pth.to_string();
    }

    debug!(
        "processing request to GET /acls/isauthz/{}/{}/{}/{:#?}",
        sub, usr, act, check_path
    );

    let _subject = get_subject_of_request(_req, pub_key).await?;
    let conn = &app_state.get_ref().db_pool.get()?;
    let result = is_authz_db(conn, sub, usr, &check_path, act)?;


    let rsp = AclAuthzCheckRsp {
        status: String::from("success"),
        message: "Result of authz check returned".to_string(),
        result,
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
        // result.push(full_path.to_string_lossy().to_string());
        result.push(full_path.file_name().unwrap().to_string_lossy().to_string());
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
    use std::env::temp_dir;
    use std::io::Write;
    use std::{collections::HashSet, ffi::OsStr};

    use actix_http::Request;
    use actix_web::App;
    use diesel::r2d2::ConnectionManager;
    use jwt_simple::algorithms::RS256PublicKey;
    use r2d2::PooledConnection;
    use reqwest::StatusCode;

    use crate::{db, make_config, models::AclDecision};

    use super::*;

    pub async fn make_test_app_state(db_name: &str) -> AppState {
        let pub_str = include_str!("../public.example");
        let db_pool = db::get_db_pool(Some(String::from(db_name)));
        AppState {
            app_version: String::from("0.1.0"),
            // put the root dir at the same location where the temporary directory will be created
            root_dir: temp_dir(),
            pub_key: RS256PublicKey::from_pem(pub_str).unwrap(),
            db_pool,
        }
    }

    pub async fn make_test_get_request(uri: &str) -> Request {
        let jwt_str = include_str!("../jwt.example");
        actix_web::test::TestRequest::get()
            .uri(uri)
            .insert_header((String::from("x-tapis-token"), jwt_str))
            .to_request()
    }

    pub fn make_test_tmp_file_system() -> std::io::Result<tempfile::TempDir> {
        // creates a temporary directory at the location returned by std::env::temp_dir()
        // typical form for unix is /tmp/.tmpzs5uel
        let temp = tempfile::TempDir::new()?;

        // create some test files for listing --
        let filename = temp.path().join("foo.txt");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "contents of the foo file";
        file.write_all(contents.as_bytes())?;

        let filename = temp.path().join("bar.txt");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "contents of the bar file";
        file.write_all(contents.as_bytes())?;

        let filename = temp.path().join("exam2");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "this exam should not be read by students";
        file.write_all(contents.as_bytes())?;

        let dirname = temp.path().join("subdir1");
        let mut _dir = std::fs::DirBuilder::new().create(dirname);

        let dirname = temp.path().join("subdir2");
        let mut _dir = std::fs::DirBuilder::new().create(dirname);

        let filename = temp.path().join("subdir1/baz.txt");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "contents of the baz file";
        file.write_all(contents.as_bytes())?;

        let filename = temp.path().join("subdir2/bam.zip");
        let mut file = std::fs::File::create(&filename)?;
        let contents = "contents of the bam file";
        file.write_all(contents.as_bytes())?;

        Ok(temp)
    }

    async fn make_acls(
        base_path: &OsStr,
        conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
    ) -> std::io::Result<()> {
        let base_path = base_path.to_str().map(|s| s.to_string()).unwrap();
        let acl = NewAclJson {
            subject: String::from("tenants@admin"),
            action: AclAction::Write,
            decision: AclDecision::Allow,
            path: format!("{}/*.txt", base_path),
            user: String::from("self"),
        };
        let _r = save_acl(
            conn,
            &acl.subject,
            &acl.action,
            &acl.path,
            &acl.user,
            &acl.decision,
            "tester_admin",
        )
        .unwrap();

        let acl = NewAclJson {
            subject: String::from("tenants@admin"),
            action: AclAction::Write,
            decision: AclDecision::Deny,
            path: format!("{}/exam*", base_path),
            user: String::from("self"),
        };
        let _r = save_acl(
            conn,
            &acl.subject,
            &acl.action,
            &acl.path,
            &acl.user,
            &acl.decision,
            "tester_admin",
        )
        .unwrap();

        let acl = NewAclJson {
            subject: String::from("tenants@admin"),
            action: AclAction::Write,
            decision: AclDecision::Allow,
            path: format!("{}/subdir2/bam.zip", base_path),
            user: String::from("self"),
        };
        let _r = save_acl(
            conn,
            &acl.subject,
            &acl.action,
            &acl.path,
            &acl.user,
            &acl.decision,
            "tester_admin",
        )
        .unwrap();

        Ok(())
    }

    #[actix_rt::test]
    async fn status_should_be_ready() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let db_name = format!("{}/status_should_be_ready.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let req = make_test_get_request("/status/ready").await;
        let resp = actix_web::test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[actix_rt::test]
    async fn files_list_dir() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let mut uri = PathBuf::from("/files/list");
        uri.push(&t_name);
        // create a database inside the temporary file system.
        let db_name = format!("{}/file_list_dir.db", td.path().to_string_lossy());
        dbg!(&db_name);
        let app_state = make_test_app_state(&db_name).await;
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let req = make_test_get_request(path_buf_to_str(&uri).unwrap()).await;

        let resp: FileListingRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        // one of the files will be the sqlite database file (files_list)_dir.db)
        let expected_result = HashSet::from([
            String::from("file_list_dir.db"),
            String::from("foo.txt"),
            String::from("bar.txt"),
            String::from("exam2"),
            String::from("subdir1"),
            String::from("subdir2"),
        ]);
        let actual_length = &resp.result.len();
        let result: HashSet<String> = resp.result.into_iter().collect();
        assert_eq!(expected_result, result);
        assert_eq!(actual_length, &6);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_list_empty() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let db_name = format!("{}/acls_list_empty.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let req = make_test_get_request("/acls").await;
        let resp: AclListingRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        let actual_length = &resp.result.len();
        assert_eq!(actual_length, &0);
        dbg!(&resp);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_list_all() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_list_all.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        let req = make_test_get_request("/acls").await;
        let resp: AclListingRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        let actual_length = &resp.result.len();
        assert_eq!(actual_length, &3);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_allow_exact() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_allow_exact.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        let uri = format!("/acls/isauthz/tenants@admin/self/Write/{}/subdir2/bam.zip", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, true);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_allow_exact_action_impl() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_allow_exact_action_impl.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        let uri = format!("/acls/isauthz/tenants@admin/self/Read/{}/subdir2/bam.zip", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, true);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }


    #[actix_rt::test]
    async fn acls_default_decision_deny() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_default_decision_deny.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        // we choose a path with no matching ACL, so the decision should be deny. 
        let uri = format!("/acls/isauthz/tenants@admin/self/Write/{}/levitation.mp3", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, false);
        // should be no matching ACL 
        assert_eq!(resp.result.acl_id, None);
        Ok(())
    }
   
    #[actix_rt::test]
    async fn acls_allow_glob() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_allow_glob.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        let uri = format!("/acls/isauthz/tenants@admin/self/Read/{}/foo.txt", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, true);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_allow_glob_subdirs() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_allow_glob_subdirs.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        // note: this test passes because of how we have set the require_literal_separator: false, 
        // see notes in  the db::check_acl_glob_for_match() function.
        let uri = format!("/acls/isauthz/tenants@admin/self/Read/{}/subdir1/sub2/sub3/foo.txt", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, true);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }

    #[actix_rt::test]
    async fn acls_deny_glob() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_deny_glob.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        let uri = format!("/acls/isauthz/tenants@admin/self/Write/{}/exam.123", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, false);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }    

    #[actix_rt::test]
    async fn acls_deny_glob_subdirs() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_deny_glob_subdirs.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        // note that in this case  the require_literal_separator: false does not actually apply; the glob in question (exam*)
        // requires a match at the beginning of the string.
        // *however*, this authz check is still false because there is *no* matching ACL; thus, the system uses the 
        // default decision. 
        let uri = format!("/acls/isauthz/tenants@admin/self/Write/{}/dir1/dir2/exam.123", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.result.allowed, false);
        // this assert verifies that no ACL matched and the system used the default decision.
        assert_eq!(resp.result.acl_id, None);
        Ok(())
    } 
    #[actix_rt::test]
    async fn acls_deny_precedence_over_allow() -> std::io::Result<()> {
        let td = make_test_tmp_file_system()?;
        let t_name = td.path().file_name().unwrap();
        let db_name = format!("{}/acls_deny_precedence_over_allow.db", td.path().to_string_lossy());
        let app_state = make_test_app_state(&db_name).await;
        let conn = app_state.db_pool.get().unwrap();
        let app = actix_web::test::init_service(
            App::new().configure(make_config(web::Data::new(app_state))),
        )
        .await;
        let _ = make_acls(t_name, &conn).await;
        // this path matches BOTH an allow ACL and a deny ACL, but deny takes precedence... 
        let uri = format!("/acls/isauthz/tenants@admin/self/Write/{}/exam.txt", t_name.to_string_lossy());
        let req = make_test_get_request(&uri).await;
        let resp: AclAuthzCheckRsp = actix_web::test::call_and_read_body_json(&app, req).await;
        dbg!(&resp.result);
        assert_eq!(resp.result.allowed, false);
        assert_ne!(resp.result.acl_id, None);
        Ok(())
    }    


}
