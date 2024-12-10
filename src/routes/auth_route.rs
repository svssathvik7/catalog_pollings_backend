use actix_web::{
    http::StatusCode, web::{Data, Path, ServiceConfig}, HttpResponse, Responder
};
use webauthn_rs::{prelude::Uuid, Webauthn};

use crate::db::{reg_state_repo::RegState, DB};

#[actix_web::post("/register/start/{username}")]
pub async fn registration_start(db: Data<DB>, username: Path<String>, webauthn: Data<Webauthn>) -> impl Responder{
    let username = username.as_str();
    let users_match = db.users.search_by_username(username).await;

    let uuid = match users_match {
        Ok(Some(_user)) => {return HttpResponse::BadRequest().status(StatusCode::BAD_REQUEST).json("User does already exist!");},
        Ok(None) => Uuid::new_v4(),
        Err(e) => {
            eprintln!("Error searching user with username {:?} : {:?}",username,e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error fetching user from db");
        }
    };

    let (ccr, reg_state) = match webauthn.start_passkey_registration(uuid, username, username, None){
        Ok(data) => data,
        Err(e) => {
            eprint!("Error creating challange and reg state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed to create registratino challange and reg state");
        }
    };

    let reg_state = match serde_json::to_value(&reg_state) {
        Ok(value) => value,
        Err(e) => {
            eprint!("Error serializing reg_state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error serializing reg_state");
        }
    };

    let new_user_reg_state = RegState{
        username: username.to_string(),
        uuid: uuid.to_string(),
        reg_state
    };

    let result = match db.reg_states.insert(new_user_reg_state).await{
        Ok(success) => {
            HttpResponse::Ok().status(StatusCode::CREATED).json(ccr)
        },
        Err(e) => {
            eprint!("Error storing reg state to db {:?}",e);
            HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error storing reg state to db ")
        }
    };
    result
}

pub fn init(cnf: &mut ServiceConfig) -> () {
    cnf.service(registration_start);
    ()
}
