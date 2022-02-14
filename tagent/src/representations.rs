use serde::Serialize;
use jwt_simple::algorithms::RS256PublicKey;


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

#[derive(Serialize)]
pub struct FileUploadRsp {
    pub message: String,
    pub status: String,
    pub version: String,
    pub result: String,
}
