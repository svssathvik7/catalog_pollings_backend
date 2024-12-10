use actix_web::{web::Data, App, HttpResponse, HttpServer};
use db::DB;
pub mod db;
#[actix_web::get("/")]
async fn home() -> HttpResponse{
    HttpResponse::Ok().json("Welcome to polling app backend")
}

#[actix_web::main]
async fn main() -> Result<(),std::io::Error>{
    let mongodb = Data::new(DB::init().await);
    HttpServer::new(
        move ||{
            App::new().service(home).app_data(mongodb.clone())
        }
    ).bind("localhost:5000")?.run().await
}
