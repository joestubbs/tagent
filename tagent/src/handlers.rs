use actix_files::NamedFile;
use actix_web::{web, Either, HttpResponse, Responder, Result};
use std::path::PathBuf;
use std::{fs, path::Path};

use actix_multipart::Multipart;
use async_std::prelude::*;
use futures::{StreamExt, TryStreamExt};

use super::representations::{AppState, ErrorRsp, FileListingRsp, FileUploadRsp, Ready};

// status endpoints ---
pub async fn ready(data: web::Data<AppState>) -> Result<impl Responder> {
    let version = &data.app_version;
    let r = Ready {
        status: String::from("success"),
        message: String::from("tagent ready."),
        result: String::from("None"),
        version: version.to_string(),
    };
    Ok(web::Json(r))
}

// acls endpoints ---
pub async fn get_all_acls() -> impl Responder {
    "todo: get_all_acls".to_string()
}

pub async fn get_acls_for_service() -> impl Responder {
    "todo: get_acls_for_service".to_string()
}

pub async fn get_acls_for_service_user() -> impl Responder {
    "todo: get_acls_for_service_user".to_string()
}

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

pub async fn list_files_path(
    data: web::Data<AppState>,
    params: web::Path<(String,)>,
) -> FileListHttpRsp {
    let version = &data.app_version;
    let root_dir = &data.root_dir;
    let params = params.into_inner();
    let path = params.0;

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
        return Either::A(HttpResponse::BadRequest().json(r));
    }
    let result = get_local_listing(full_path);

    let r = FileListingRsp {
        status: String::from("success"),
        message: String::from("File listing retrieved successfully"),
        version: version.to_string(),
        result,
    };
    Either::B(web::Json(r))
}

type FileContentsHttpRsp = Either<HttpResponse, Result<NamedFile>>;

pub async fn get_file_contents_path(
    data: web::Data<AppState>,
    params: web::Path<(String,)>,
) -> FileContentsHttpRsp {
    let version = &data.app_version;
    let root_dir = &data.root_dir;
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
        let r = ErrorRsp {
            status: String::from("error"),
            message,
            version: version.to_string(),
            result: String::from("none"),
        };
        return Either::A(HttpResponse::BadRequest().json(r));
    }
    Either::B(Ok(NamedFile::open(full_path).unwrap()))
}

pub async fn save_file(mut payload: Multipart, full_path: &str) -> Option<String> {
    let mut filepath = String::from("empty");
    // iterate over multipart stream
    while let Ok(Some(mut field)) = payload.try_next().await {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_type = field
            .content_disposition()
            .ok_or(actix_web::error::ParseError::Incomplete)
            .unwrap();

        let filename = content_type
            .get_filename()
            .ok_or(actix_web::error::ParseError::Incomplete)
            .unwrap();
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

pub async fn post_file_contents_path(
    data: web::Data<AppState>,
    params: web::Path<(String,)>,
    payload: Multipart,
) -> FileUploadHttpRsp {
    let version = &data.app_version;
    let root_dir = &data.root_dir;
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
        return Either::A(HttpResponse::BadRequest().json(r));
    };
    let full_path_s = path_buf_to_string(full_path).unwrap();
    let upload_path = save_file(payload, &full_path_s).await;

    let r = FileUploadRsp {
        status: String::from("success"),
        message: format!("file uploaded to {:?} successfully.", upload_path),
        result: String::from("none"),
        version: version.to_string(),
    };

    Either::B(web::Json(r))
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
