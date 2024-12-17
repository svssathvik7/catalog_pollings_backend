use std::vec;

use actix_web::{
    http::StatusCode, web::{Data, Json, Path, ServiceConfig}, HttpResponse, Responder
};
use mongodb::bson::{doc, oid::ObjectId};
use nanoid::nanoid;

use crate::{
    db::{options_repo::Option, polls_repo::Poll, DB},
    models::poll_api_model::NewPollRequest,
    utils::jwt::JWT,
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
        let new_option = Option {
            text: option.text,
            votes_count: 0,
            voters: Vec::new(),
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
    let filter = doc! {"username": poll_data.ownername};
    let owner_id = match db.users.query_by_filter(filter).await {
        Ok(Some(owner_match)) => {
            if let Some(id) = owner_match.id {
                id
            } else {
                session.abort_transaction().await.unwrap();
                return HttpResponse::InternalServerError()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Failed fetching owner id!");
            }
        }
        Ok(None) => {
            return HttpResponse::Forbidden()
                .status(StatusCode::FORBIDDEN)
                .body("Login to create polls!");
        }
        Err(e) => {
            eprint!("Error finding owner id {:?}", e);
            session.abort_transaction().await.unwrap();
            return HttpResponse::Forbidden()
                .status(StatusCode::FORBIDDEN)
                .body("Login to create polls!");
        }
    };
    let new_poll = Poll {
        id: nanoid!(),
        title,
        options: option_ids,
        owner_id,
        is_open: true
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

#[actix_web::get("/{id}")]
pub async fn get_poll(id: Path<String>,db:Data<DB>) -> impl Responder{
    let poll_data = match db.polls.get(id.as_str()).await{
        Ok(Some(poll)) => poll,
        Ok(None) => {
            return HttpResponse::BadRequest()
            .status(StatusCode::NOT_FOUND)
            .body("No poll found!");
        },
        Err(e)=>{
            eprint!("Error finding poll {:?}",e);
            return HttpResponse::InternalServerError()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Failed fetching poll :(");
        }
    };
    HttpResponse::Ok()
    .status(StatusCode::ACCEPTED)
    .json(poll_data)
}

#[actix_web::get("/{id}/close")]
pub async fn close_poll(id: Path<String>,db:Data<DB>) -> impl Responder{
    let _close_poll = match db.polls.close_poll(id.as_str()).await {
        Ok(_closed) => {
            return HttpResponse::Ok().status(StatusCode::ACCEPTED).json("Poll closed!");
        },
        Err(e) =>{
            eprint!("Error deleting poll {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed closing poll, try again later!");
        }
    };
}

#[actix_web::get("/{id}/reset")]
pub async fn reset_poll(id: Path<String>,db:Data<DB>) -> impl Responder{
    let _reset_poll = match db.clone().polls.reset_poll(id.as_str(), &db.into_inner()).await {
        Ok(_reset) => {
            return HttpResponse::Ok().status(StatusCode::ACCEPTED).json("Poll reset!");
        },
        Err(e) =>{
            eprint!("Error resetting poll {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed resetting poll, try again later!");
        }
    };
}

pub fn init(cnf: &mut ServiceConfig) {
    cnf.service(create_poll).service(get_poll).service(close_poll);
    ()
}
