use std::sync::Mutex;

use crate::sse::Broadcaster;
use actix_web::{
    web::{Data, ServiceConfig},
    HttpResponse, Responder,
};

#[actix_web::get("/create-client")]
pub async fn create_sse_client(broadcaster: Data<Mutex<Broadcaster>>) -> impl Responder {
    let mut broadcaster = broadcaster.lock().unwrap();
    let client = broadcaster.new_client();
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(client)
}

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(create_sse_client);
}
