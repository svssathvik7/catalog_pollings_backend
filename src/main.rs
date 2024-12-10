use actix_web::{App, HttpResponse, HttpServer};
pub mod db;
#[actix_web::get("/")]
async fn home() -> HttpResponse{
    HttpResponse::Ok().json("Welcome to polling app backend")
}

#[actix_web::main]
async fn main() -> Result<(),std::io::Error>{
    HttpServer::new(
        ||{
            App::new().service(home)
        }
    ).bind("localhost:5000")?.run().await
}
