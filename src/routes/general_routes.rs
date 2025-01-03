use actix_web::{
    web::{self, Data, ServiceConfig},
    HttpResponse, Responder,
};
use serde::Deserialize;

use crate::db::DB;

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[actix_web::get("/live")]
pub async fn get_live_polls(
    db: Data<DB>,
    web::Query(params): web::Query<PaginationParams>,
) -> impl Responder {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(2);

    // Fetch polls
    let polls = match db.polls.get_live_polls(page, per_page).await {
        Ok(polls) => polls,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to fetch closed polls");
        }
    };

    let total_polls = match db.polls.count_live_polls().await {
        Ok(count) => count,
        Err(_) => 0,
    };

    // Prepare response with pagination info
    HttpResponse::Ok().json(serde_json::json!({
        "polls": polls,
        "page": page,
        "per_page": per_page,
        "total_polls": total_polls,
        "total_pages": (total_polls as f64 / per_page as f64).ceil() as u64
    }))
}

#[actix_web::get("/closed")]
pub async fn get_closed_polls(
    db: Data<DB>,
    web::Query(params): web::Query<PaginationParams>,
) -> impl Responder {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(10);

    // Fetch polls
    let polls = match db.polls.get_closed_polls(page, per_page).await {
        Ok(polls) => polls,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to fetch closed polls");
        }
    };

    let total_polls = match db.polls.count_closed_polls().await {
        Ok(count) => count,
        Err(_) => 0,
    };

    // Prepare response with pagination info
    HttpResponse::Ok().json(serde_json::json!({
        "polls": polls,
        "page": page,
        "per_page": per_page,
        "total_polls": total_polls,
        "total_pages": (total_polls as f64 / per_page as f64).ceil() as u64
    }))
}

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(get_live_polls).service(get_closed_polls);
    ()
}
