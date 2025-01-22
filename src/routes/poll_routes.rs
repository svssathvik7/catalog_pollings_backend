use std::{collections::HashMap, sync::Mutex};

use actix_web::{
    http::StatusCode,
    web::{self, Data, Json, Path, ServiceConfig},
    HttpResponse, Responder,
};
use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use nanoid::nanoid;
use serde::Deserialize;

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<u64>,
    per_page: Option<u64>,
    sort_by: Option<String>,
    sort_order: Option<i8>,
}

use crate::{
    db::{options_repo::OptionModel, polls_repo::Poll, DB},
    models::poll_api_model::{NewPollRequest, PollResults},
    sse::Broadcaster,
};

#[actix_web::post("/new")]
pub async fn create_poll(req: Json<NewPollRequest>, db: Data<DB>) -> impl Responder {
    let poll_data = req.into_inner();
    let mut session = db.client.start_session().await.unwrap();
    session.start_transaction().await.unwrap();
    let title = poll_data.title;
    let mut option_ids: Vec<ObjectId> = Vec::new();
    let options = if poll_data.options.len() >= 2 {
        poll_data.options
    } else {
        return HttpResponse::BadRequest()
            .status(StatusCode::BAD_REQUEST)
            .json("Minimum two options are required to create the poll!");
    };
    let mut option_inserted = true;
    for option in options {
        let new_option = OptionModel {
            _id: ObjectId::new(),
            text: option.text,
            votes_count: 0,
        };
        option_inserted = option_inserted
            && match db.options.insert(new_option).await {
                Ok(inserted_option) => {
                    if let Some(inserted_id) = inserted_option.inserted_id.as_object_id() {
                        option_ids.push(inserted_id); // Push the ObjectId into the vector
                        true
                    } else {
                        session.abort_transaction().await.unwrap();
                        eprintln!("Insert succeeded, but no ObjectId returned!");
                        false
                    }
                }
                Err(e) => {
                    session.abort_transaction().await.unwrap();
                    eprint!("Error writing option! {:?}", e);
                    false
                }
            };
    }
    let new_poll = Poll {
        id: nanoid!(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        title,
        options: option_ids,
        owner_id: poll_data.ownername,
        is_open: true,
        voters: Vec::new(),
    };
    let _poll_insert_result = match db.polls.insert(new_poll).await {
        Ok(inserted_poll) => inserted_poll,
        Err(e) => {
            eprint!("Error inserting poll {:?}", e);
            session.abort_transaction().await.unwrap();
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Error creating poll!");
        }
    };
    session.commit_transaction().await.unwrap();
    HttpResponse::Ok()
        .status(StatusCode::CREATED)
        .body("Successfully created poll!")
}

#[actix_web::post("/{id}")]
pub async fn get_poll(
    id: Path<String>,
    db: Data<DB>,
    Json(username): Json<HashMap<String, String>>,
) -> impl Responder {
    let username = match username.get("username") {
        Some(username) => username,
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::NOT_FOUND)
                .body("Need username!");
        }
    };
    let poll_data = match db.polls.get(id.as_str(), &username).await {
        Ok(poll_response) => poll_response,
        Err(e) => {
            eprint!("Error finding poll {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("Failed fetching poll :(");
        }
    };
    HttpResponse::Ok()
        .status(StatusCode::ACCEPTED)
        .json(poll_data)
}

#[actix_web::post("/{id}/close")]
pub async fn close_poll(
    id: Path<String>,
    db: Data<DB>,
    Json(req): Json<HashMap<String, String>>,
) -> impl Responder {
    let username = match req.get("username") {
        Some(username) => username.clone(),
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::FORBIDDEN)
                .json("Owner username required!");
        }
    };
    let _close_poll = match db.polls.close_poll(id.as_str(), &username).await {
        Ok(_closed) => {
            return HttpResponse::Ok()
                .status(StatusCode::ACCEPTED)
                .json("Poll closed!");
        }
        Err(e) => {
            eprint!("Error deleting poll {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Failed closing poll, try again later!");
        }
    };
}

#[actix_web::post("/{id}/delete")]
pub async fn delete_poll(
    id: Path<String>,
    db: Data<DB>,
    Json(req): Json<HashMap<String, String>>,
) -> impl Responder {
    let username = match req.get("username") {
        Some(username) => username.clone(),
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::FORBIDDEN)
                .json("Owner username required!");
        }
    };
    let _is_poll_deleted = match db.polls.delete(id.as_str(), &username).await {
        Ok(_) => {
            return HttpResponse::Ok()
                .status(StatusCode::ACCEPTED)
                .json("Poll deleted");
        }
        Err(e) => {
            eprintln!("Error deleting the poll! {:?}", e);
            return HttpResponse::BadRequest()
                .status(StatusCode::BAD_REQUEST)
                .json("Failed deleting the poll!");
        }
    };
}

#[actix_web::post("/{id}/reset")]
pub async fn reset_poll(
    id: Path<String>,
    db: Data<DB>,
    Json(req): Json<HashMap<String, String>>,
    broadcaster: Data<Mutex<Broadcaster>>,
) -> impl Responder {
    let username = match req.get("username") {
        Some(username) => username.clone(),
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::FORBIDDEN)
                .json("Owner username required!");
        }
    };

    match db.polls.reset_poll(id.as_str(), &db, &username).await {
        Ok(_) => {
            let poll_result_data = match db.polls.get_poll_results(&id).await.unwrap() {
                Some(poll_result) => poll_result,
                None => PollResults {
                    id: "1234".to_string(),
                    title: "Never reaches".to_string(),
                    options: Vec::new(),
                    total_votes: 0,
                },
            };
            broadcaster
                .lock()
                .unwrap()
                .send_poll_results(&poll_result_data);
            return HttpResponse::Ok()
                .status(StatusCode::ACCEPTED)
                .json("Poll reset successfully!");
        }
        Err(e) => {
            eprintln!("Error resetting poll! {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Couldn't reset poll!");
        }
    }
}

#[actix_web::post("/{id}/vote")]
pub async fn cast_vote(
    db: Data<DB>,
    id: Path<String>,
    broadcaster: Data<Mutex<Broadcaster>>,
    Json(req): Json<HashMap<String, String>>,
) -> impl Responder {
    // 1. Extract and validate username
    let username = match req.get("username") {
        Some(username) => username.to_string(),
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::BAD_REQUEST)
                .json("Failed casting vote! Require username!!");
        }
    };

    // 2. Extract and validate option ID
    let option_id = match req.get("optionId") {
        Some(option_id) => {
            // Convert option_id string to ObjectId
            match ObjectId::parse_str(option_id) {
                Ok(id) => id,
                Err(_) => {
                    return HttpResponse::BadRequest()
                        .status(StatusCode::BAD_REQUEST)
                        .json("Invalid option ID format!");
                }
            }
        }
        None => {
            return HttpResponse::BadRequest()
                .status(StatusCode::BAD_REQUEST)
                .json("Failed casting vote! Require vote option!!");
        }
    };

    // 4. Attempt to cast vote
    match db
        .polls
        .add_vote(&id, username.clone(), option_id, &db)
        .await
    {
        Ok(true) => {
            let poll_result_data = match db.polls.get_poll_results(&id).await.unwrap() {
                Some(poll_result) => poll_result,
                None => PollResults {
                    id: "1234".to_string(),
                    title: "Never reaches".to_string(),
                    options: Vec::new(),
                    total_votes: 0,
                },
            };
            broadcaster
                .lock()
                .unwrap()
                .send_poll_results(&poll_result_data);
            return HttpResponse::Ok()
                .status(StatusCode::ACCEPTED)
                .json("Vote recorded successfully!");
        }
        Ok(false) => HttpResponse::BadRequest()
            .status(StatusCode::BAD_REQUEST)
            .json("Unable to cast vote. Poll might be closed or you've already voted."),
        Err(e) => {
            eprintln!("Vote casting error: {:?}", e);
            HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Internal error while casting vote")
        }
    }
}

#[actix_web::get("/user/{username}")] // GET /polls/user
pub async fn get_user_polls(
    db: Data<DB>,
    web::Query(params): web::Query<PaginationParams>,
    username: Path<String>,
) -> impl Responder {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(5);
    let sort_by = params.sort_by.unwrap_or("created_at".to_string());
    let sort_order = params.sort_order.unwrap_or(-1);
    let user_polls = match db
        .polls
        .get_polls_by_username(
            &username.to_string(),
            page,
            per_page,
            sort_by.as_str(),
            sort_order,
        )
        .await
    {
        Ok(polls) => polls,
        Err(e) => {
            eprintln!("Error fetching user polls: {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Failed to fetch user polls");
        }
    };

    let total_polls = db
        .polls
        .count_polls_by_username(&username.to_string())
        .await
        .unwrap();

    HttpResponse::Ok()
        .status(StatusCode::OK)
        .json(serde_json::json!({
            "polls": user_polls,
            "page": page,
            "per_page": per_page,
            "total_polls": total_polls,
            "total_pages": (total_polls as f64 / per_page as f64).ceil() as u64
        }))
}

#[actix_web::get("/{id}/results")]
pub async fn get_poll_result(db: Data<DB>, id: Path<String>) -> impl Responder {
    let poll_id = id.as_str();
    match db.polls.get_poll_results(poll_id).await {
        Ok(poll_result) => {
            return HttpResponse::Ok().status(StatusCode::OK).json(poll_result);
        }
        Err(e) => {
            eprintln!("Error fetching poll results! {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Error fetching poll results");
        }
    };
}

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(create_poll)
        .service(get_poll)
        .service(cast_vote)
        .service(close_poll)
        .service(get_user_polls)
        .service(reset_poll)
        .service(delete_poll)
        .service(get_poll_result);
    ()
}
