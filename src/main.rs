use actix_cors::Cors;
use actix_web::{
    middleware::from_fn,
    web::{scope, Data},
    App, HttpServer,
};
use db::DB;
use dotenv::dotenv;
use log::error;
use middlewares::authenticate::authenticate_user;
use routes::{auth_routes, general_routes, poll_routes, sse_route};
use sse::Broadcaster;
use std::env;
use utils::jwt::JWT;
use webauthn::config_webauthn;
pub mod db;
pub mod middlewares;
pub mod models;
pub mod routes;
pub mod sse;
pub mod utils;
pub mod webauthn;

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv().ok();
    let client_origin = "https://catalog-pollings.vercel.app";
    let mongodb = Data::new(DB::init().await);
    let webauthn = Data::new(config_webauthn().unwrap());
    let jwt = Data::new(JWT::init());
    let broadcaster = Broadcaster::create();
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allowed_origin(&client_origin)
                    .supports_credentials(),
            )
            .service(
                scope("/api")
                    .service(scope("/p").configure(general_routes::init))
                    .service(scope("/auth").configure(auth_routes::init))
                    .service(scope("/sse").configure(sse_route::init))
                    .service(
                        scope("")
                            .wrap(from_fn(authenticate_user))
                            .service(scope("/polls").configure(poll_routes::init)),
                    ),
            )
            .app_data(mongodb.clone())
            .app_data(webauthn.clone())
            .app_data(jwt.clone())
            .app_data(broadcaster.clone())
    })
    .bind(("0.0.0.0", 5000))?
    .run()
    .await
}
