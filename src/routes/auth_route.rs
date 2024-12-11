use actix_web::{
    http::StatusCode, web::{Data, Json, Path, ServiceConfig}, HttpResponse, Responder
};
use webauthn_rs::{prelude::{PasskeyRegistration, RegisterPublicKeyCredential, Uuid}, Webauthn};

use crate::db::{reg_state_repo::RegState, users_repo::User, DB};

#[actix_web::post("/register/start/{username}")]
pub async fn start_registration(db: Data<DB>, username: Path<String>, webauthn: Data<Webauthn>) -> impl Responder{
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
    // serialize the reg state
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
            println!("{:?}",success);
            HttpResponse::Ok().status(StatusCode::CREATED).json(ccr)
        },
        Err(e) => {
            eprint!("Error storing reg state to db {:?}",e);
            HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error storing reg state to db ")
        }
    };
    result
}

#[actix_web::post("/register/finish/{username}")]
pub async fn finish_registration(db:Data<DB>,webauthn: Data<Webauthn>, username: Path<String>, request: Json<RegisterPublicKeyCredential>) -> impl Responder{
    // println!("{:?}",request);
    let username = username.as_str();
    println!("{}",username);
    let reg_state_match = match db.reg_states.find_by_username(username).await {
        Ok(Some(document_match)) => document_match.reg_state,
        Ok(None) => {return HttpResponse::Forbidden().status(StatusCode::FORBIDDEN).json("No initiation of registration found!");},
        Err(e) => {
            eprint!("Failed getting reg state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed getting reg state");
        }
    };

    let deserialize_reg_state: PasskeyRegistration = match serde_json::from_value(reg_state_match) {
        Ok(data) => data,
        Err(e) => {
            eprint!("Failed deserializing registration state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed deserializing registration state");
        }
    };

    let pass_key = match webauthn.finish_passkey_registration(&request, &deserialize_reg_state){
        Ok(key) => key,
        Err(e) => {
            eprint!("Failed generating the passkey {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed generating the passkey");
        }
    };

    // serialize the key
    let sk = match serde_json::to_value(pass_key){
        Ok(data) => data,
        Err(e) => {
            eprint!("Failed serializing the key {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed serializing the key");
        }
    };

    let new_user = User{
        username: username.to_string(),
        uuid: Uuid::new_v4().to_string(),
        sk
    };

    let result = match db.users.insert(new_user).await {
        Ok(_inserted_user) => HttpResponse::Ok().status(StatusCode::CREATED).json(format!("Successfully registered {}",username)),
        Err(e) => {
            eprint!("Failed registering the user! {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed registering the user, try again after sometime");
        }
    };

    result
}


pub fn init(cnf: &mut ServiceConfig) -> () {
    cnf.service(start_registration).service(finish_registration);
    ()
}
