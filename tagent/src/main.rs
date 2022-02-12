use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;

mod auth;
mod handlers;
mod models;
mod representations;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let app_version = String::from("0.1.0");
    let root_dir = String::from("/home/jstubbs/projects");
    let pub_key = String::from("-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAieWoWm/7AZPheleSJSXR/CS9vnr2m7qsjzvqv7PXr5AOxpw+eYn5h1/7lBqludzle4fI8ai/mv2WsTEC7C3HuIF5D+EuCQtXe89YPI8e4Q/gc660vWhG3ZYA5UrZyAOAJvDvcd/N8ZCjyW8fZ5tYsMlcWf6m9d29QLtLc8kIIZJiFuQcfiq5NiaB5tYU6zOQzO6fYUO44egni1DH6spm0btqobIsNQauunXSuZD3lLwXGnuS1VE+3pPEIFeAq0tnQcuJxsUZIRbiWRgAnNHCFoxeB3kMysKUr1IMjqlUlTBgDbCvfn8RJxQUeMgEJygsa/m9xHzfX3IoAm4NfvsEPwIDAQAB\n-----END PUBLIC KEY-----");

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
            // declare app state
            .app_data(actix_app_state.clone())
            // declare routes
            .service(
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
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
