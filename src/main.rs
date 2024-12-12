use actix_cors::Cors;
use actix_web::{
    web::{scope, Data},
    App, HttpResponse, HttpServer,
};
use db::DB;
use routes::auth_route;
use utils::jwt::JWT;
use webauthn::config_webauthn;
pub mod db;
pub mod utils;
pub mod routes;
pub mod webauthn;
#[actix_web::get("/")]
async fn home() -> HttpResponse {
    HttpResponse::Ok().json("Welcome to polling app backend")
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    let mongodb = Data::new(DB::init().await);
    let webauthn = Data::new(config_webauthn().unwrap());
    let jwt = Data::new(JWT::init());
    HttpServer::new(move || {
        App::new()
            .service(home)
            .service(scope("/auth").configure(auth_route::init)).wrap(
                Cors::default().allow_any_header().allow_any_method().allow_any_origin().supports_credentials()
            )
            .app_data(mongodb.clone())
            .app_data(webauthn.clone()).app_data(jwt.clone())
    })
    .bind("localhost:5000")?
    .run()
    .await
}
