use actix_web::{web::{scope, Data}, App, HttpResponse, HttpServer};
use db::DB;
use routes::auth_route;
pub mod webauthn;
pub mod db;
pub mod routes;
#[actix_web::get("/")]
async fn home() -> HttpResponse{
    HttpResponse::Ok().json("Welcome to polling app backend")
}

#[actix_web::main]
async fn main() -> Result<(),std::io::Error>{
    let mongodb = Data::new(DB::init().await);
    HttpServer::new(
        move ||{
            App::new().service(home).app_data(mongodb.clone()).service(scope("/auth").configure(auth_route::init))
        }
    ).bind("localhost:5000")?.run().await
}
