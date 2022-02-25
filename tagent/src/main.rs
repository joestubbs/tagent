use actix_web::middleware::Logger;
use actix_web::web::ServiceConfig;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use log::info;

mod auth;
mod config;
mod handlers;
mod models;
mod representations;

fn make_config(app_data: web::Data<representations::AppState>) -> impl FnOnce(&mut ServiceConfig) {
    |cfg: &mut ServiceConfig| {
        cfg.app_data(app_data).service(
            //
            web::scope("")
                // status routes ----
                .service(handlers::ready)
                // acls routes ----
                .service(handlers::get_all_acls)
                .service(handlers::get_acls_for_service)
                .service(handlers::get_acls_for_service_user)
                .service(handlers::is_authz_service_user_path)
                // files routes ----
                .service(handlers::list_files_path)
                .service(handlers::get_file_contents_path)
                .service(handlers::post_file_contents_path),
        );
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

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
            // set up application logging
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .configure(make_config(actix_app_state.clone()))
    })
    .bind(format!("{}:{}", settings.address, settings.port))?
    .run()
    .await
}
