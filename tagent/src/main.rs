use actix_web::middleware::{Logger, ErrorHandlerResponse, ErrorHandlers};
use actix_web::web::{JsonConfig, ServiceConfig, PathConfig};
use actix_web::http::StatusCode;
use actix_web::{web, dev, App, HttpServer, Result};
use actix_web::{error, HttpResponse};

#[macro_use]
extern crate diesel;

use dotenv::dotenv;
use log::info;

mod auth;
mod config;
mod db;
mod handlers;
mod models;
mod representations;
mod schema;

fn make_config(app_data: web::Data<representations::AppState>) -> impl FnOnce(&mut ServiceConfig) {
    |cfg: &mut ServiceConfig| {
        cfg.app_data(app_data).service(
            //
            web::scope("")
                // status routes ----
                .service(handlers::ready)
                // acls routes ----
                .service(handlers::create_acl)
                .service(handlers::get_all_acls)
                .service(handlers::get_acl_by_id)
                .service(handlers::delete_acl_by_id)
                .service(handlers::update_acl_by_id)
                .service(handlers::get_acls_for_subject)
                .service(handlers::get_acls_for_subject_user)
                .service(handlers::is_authz_subject_user_action_path)
                // files routes ----
                .service(handlers::list_files_path)
                .service(handlers::get_file_contents_path)
                .service(handlers::post_file_contents_path)
                .default_service(web::route().to(handlers::not_found))
        );
    }
}


fn make_error_json_rsp<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> 
where B: std::fmt::Debug + actix_web::body::MessageBody {

    let (req, res) = res.into_parts();
    let msg = res.body().clone();
    let body = format!("{{\"status\": \"error\", \"message\": {:#?}, \"result\": \"none\"}}", msg);
    // let res = res.set_body(r#"{"status": "error", "message": "", "result": "none"}"#.to_owned());
    let res = res.set_body(body);
    let res = dev::ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();

    Ok(ErrorHandlerResponse::Response(res))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    // custom `Json` extractor configuration
    let json_cfg = JsonConfig::default()
    // limit request payload size
    .limit(4096*1024)
    // use custom error handler
    .error_handler(|err, req| {
        representations::TagentError::new_with_version(err.to_string()).into()
    });

    let path_cfg = PathConfig::default()
    // use custom error handler
    .error_handler(|err, req| {
        representations::TagentError::new_with_version(err.to_string()).into()
    });

    let settings = crate::config::TagentConfig::from_sources()?;
    let app_version = String::from(env!("CARGO_PKG_VERSION"));
    let root_dir = settings.root_directory.clone();
    info!("tagent version {}", app_version);
    info!("tagent running with root directory: {:?}", &root_dir);
    info!("tagent serving at {}:{}", settings.address, settings.port);
    let pub_key = settings.get_public_key().await?;
    let app_state = representations::AppState {
        app_version,
        root_dir,
        pub_key,
    };

    let actix_app_state = web::Data::new(app_state);

    HttpServer::new(move || {
        dotenv().ok();
        App::new()
            // set up JSON errors
            .app_data(json_cfg.clone())
            .app_data(path_cfg.clone())
            // // set up application logging
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .configure(make_config(actix_app_state.clone()))
            // .default_service(handlers::not_found)
    })
    .bind(format!("{}:{}", settings.address, settings.port))?
    .run()
    .await
}
