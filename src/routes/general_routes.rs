use std::sync::{Arc, Mutex};

use actix_web::{
    http::StatusCode,
    web::{self, Data, ServiceConfig},
    Responder,
};
use serde::Deserialize;

use crate::{db::DB, utils::json_responder::Response};

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[actix_web::get("/live")]
pub async fn get_live_polls(
    db: Data<Arc<Mutex<DB>>>,
    web::Query(params): web::Query<PaginationParams>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(2);

    // Fetch polls
    let polls = match db.polls.get_live_polls(page, per_page).await {
        Ok(polls) => polls,
        Err(e) => {
            eprintln!("Error fetching live polls {:?}", e);
            return Response::<String>::error(
                "Failed fetching live polls!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let total_polls = match db.polls.count_live_polls().await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Error fetching polls count! {:?}", e);
            0
        }
    };

    // Prepare response with pagination info
    Response::ok(
        serde_json::json!({
            "polls": polls,
            "page": page,
            "per_page": per_page,
            "total_polls": total_polls,
            "total_pages": (total_polls as f64 / per_page as f64).ceil() as u64
        }),
        StatusCode::OK,
    )
}

#[actix_web::get("/closed")]
pub async fn get_closed_polls(
    db: Data<Arc<Mutex<DB>>>,
    web::Query(params): web::Query<PaginationParams>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(10);

    // Fetch polls
    let polls = match db.polls.get_closed_polls(page, per_page).await {
        Ok(polls) => polls,
        Err(e) => {
            eprintln!("Error fetching closed polls {:?}", e);
            return Response::<String>::error(
                "Failed fetching closed polls!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let total_polls = match db.polls.count_closed_polls().await {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Error fetching closed polls count {:?}", e);
            0
        }
    };

    // Prepare response with pagination info
    Response::ok(
        serde_json::json!({
            "polls": polls,
            "page": page,
            "per_page": per_page,
            "total_polls": total_polls,
            "total_pages": (total_polls as f64 / per_page as f64).ceil() as u64
        }),
        StatusCode::OK,
    )
}

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(get_live_polls).service(get_closed_polls);
    ()
}
