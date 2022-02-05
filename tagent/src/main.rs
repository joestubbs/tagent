use actix_web::{web, App, HttpServer};

mod handlers;
mod models;
mod representations;



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=debug");

    HttpServer::new(|| {
        App::new()
        .data(representations::AppState {
            app_name: String::from("tagent"),
            app_version: String::from("0.1.0"),
            root_dir: String::from("/home/jstubbs/projects"),
        })
          .service(
            // 
            web::scope("")                
                // status routes ----
                .route("/status/ready", web::get().to(handlers::ready))
 
                // acls routes ----
                .route("/acls/all", web::get().to(handlers::get_all_acls))
                .route("/acls/{service}", web::get().to(handlers::get_acls_for_service))
                .route("/acls/{service}/{user}", web::get().to(handlers::get_acls_for_service_user))
                .route("/acls/isauthz/{service}/{user}/{path:.*}", web::get().to(handlers::is_authz_service_user_path))

                // files routes ----
                .route("/files/list/{path:.*}", web::get().to(handlers::list_files_path))
                .route("/files/contents/{path:.*}", web::get().to(handlers::get_file_contents_path))
                .route("/files/contents/{path:.*}", web::post().to(handlers::post_file_contents_path))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
