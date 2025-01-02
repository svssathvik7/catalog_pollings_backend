use std::{collections::HashMap, hash::Hash, vec};

use actix_web::{
    http::StatusCode,
    web::{Data, Json, Path, ServiceConfig},
    HttpResponse, Responder,
};
use chrono::Utc;
use mongodb::bson::{doc, oid::ObjectId};
use nanoid::nanoid;

use crate::{
    db::{options_repo::OptionModel, polls_repo::Poll, DB},
    models::poll_api_model::NewPollRequest,
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
            votes_count: 0
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
        voters: Vec::new()
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
pub async fn get_poll(id: Path<String>, db: Data<DB>, Json(username): Json<HashMap<String,String>>) -> impl Responder {
    let username = match username.get("username"){
        Some(username) => username,
        None => {
            return HttpResponse::BadRequest()
            .status(StatusCode::NOT_FOUND)
            .body("Need username!");
        }
    };
    println!("poll id: {:?}", id.as_str());
    let poll_data = match db.polls.get(id.as_str(),&username).await {
        Ok(poll_response) => {
            poll_response
        },
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

#[actix_web::post("/{id}/reset")]
pub async fn reset_poll(
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
    let _reset_poll = match db
        .clone()
        .polls
        .reset_poll(id.as_str(), &db.into_inner(), &username)
        .await
    {
        Ok(_reset) => {
            return HttpResponse::Ok()
                .status(StatusCode::ACCEPTED)
                .json("Poll reset!");
        }
        Err(e) => {
            eprint!("Error resetting poll {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Failed resetting poll, try again later!");
        }
    };
}

#[actix_web::post("/{id}/vote")]
pub async fn cast_vote(
    db: Data<DB>,
    id: Path<String>,
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
    match db.polls.add_vote(&id, username, option_id, &db).await {
        Ok(true) => HttpResponse::Ok()
            .status(StatusCode::ACCEPTED)
            .json("Vote recorded successfully!"),
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

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(create_poll)
        .service(get_poll)
        .service(cast_vote)
        .service(close_poll);
    ()
}
