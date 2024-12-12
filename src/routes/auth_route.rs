use actix_web::{
    cookie::{time::Duration, Cookie, SameSite}, http::StatusCode, web::{Data, Json, Path, ServiceConfig}, HttpResponse, Responder
};
use webauthn_rs::{prelude::{Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential, Uuid}, Webauthn};

use crate::{db::{auth_state_repo::AuthState, reg_state_repo::RegState, users_repo::User, DB}, utils::jwt::JWT};
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
    let _does_reg_state_exist = match db.reg_states.is_exists(username).await {
        Ok(data) => {
            if data{
                data
            }else{
                return HttpResponse::NotFound().status(StatusCode::NOT_FOUND).json("No registration was initiated");
            }
        },
        Err(e) => {
            eprint!("Error registering {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("No registration was initiated due to internal server error");
        }
    };
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

    let _del_reg_state = match db.reg_states.delete_by_username(username).await {
        Ok(response) => response,
        Err(e) => {
            eprint!("Error deleting the reg state of {:?} {:?}",username,e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed deleting the user reg state, try again after sometime");
        }
    };

    result
}

#[actix_web::post("/login/start/{username}")]
pub async fn start_authentication(username:Path<String>, db: Data<DB>, webauthn: Data<Webauthn>) -> impl Responder{
    let username = username.as_str();
    let _does_user_exist = match db.users.is_exists(username).await{
        Ok(boolean_response) => {
            if boolean_response{
                println!("{}",boolean_response);
                boolean_response
            }else{
                return HttpResponse::NotFound().status(StatusCode::NOT_FOUND).json("No user found to sign in");
            }
        },
        Err(e) => {
            eprint!("No user found to sign in! {:?}",e);
            return HttpResponse::NotFound().status(StatusCode::NOT_FOUND).json("No user found to sign in");
        }
    };

    let user_sk = match db.users.search_by_username(username).await{
        Ok(Some(user)) => user.sk,
        Ok(None) => {
            // control reaches this line only if a user exists. Hence a None might indicate a db controller issue
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error finding a user");
        }
        Err(e) => {
            eprint!("Error searching user! {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error finding a user");
        }
    };

    // for now just a single passkey
    let sk: Vec<Passkey> = match serde_json::from_value(user_sk){
        Ok(key) => vec![key],
        Err(e) => {
            eprint!("Error deserializing sk {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error deserializing sk");
        }
    };

    let (rcr,auth_state) = match webauthn.start_passkey_authentication(&sk){
        Ok(data) => data,
        Err(e) => {
            eprint!("Error generating auth challange {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error generating auth challange");
        }
    };

    let serial_auth_state = match serde_json::to_value(auth_state) {
        Ok(serial_auth_state) => serial_auth_state,
        Err(e) => {
            eprint!("Error serialzing auth state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error serialzing auth state");
        }
    };

    let auth_state_entry = AuthState{
        auth_state: serial_auth_state,
        username: username.to_string()
    };

    let _result = match db.auth_states.insert(auth_state_entry).await {
        Ok(inserted) => inserted,
        Err(e) => {
            eprint!("Error writing auth state to db {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error writing auth state to db");
        }
    };
    return HttpResponse::Ok().status(StatusCode::CREATED).json(rcr);
}

#[actix_web::post("/login/finish/{username}")]
pub async fn finish_authentication(username:Path<String>,db:Data<DB>,webauthn: Data<Webauthn>,req:Json<PublicKeyCredential>, jwt: Data<JWT>) -> impl Responder{
    let username = username.as_str();
    let _does_auth_state_exist = match db.auth_states.is_exists(username).await {
        Ok(data) => {
            if data{
                data
            }else{
                return HttpResponse::NotFound().status(StatusCode::NOT_FOUND).json("No user found to finish authentication");
            }
        },
        Err(e) => {
            eprint!("Error authenticating {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("No user found to finish authentication");
        }
    };

    let user_auth_state = match db.auth_states.find_by_username(username).await {
        Ok(Some(data)) => data.auth_state,
        Ok(None) => {return HttpResponse::NotFound().status(StatusCode::NOT_FOUND).json("No user found to finish authentication");},
        Err(e) =>{
            eprint!("Error fetching auth state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error fetching auth state");
        }
    };

    let deserialized_as: PasskeyAuthentication = match serde_json::from_value(user_auth_state) {
        Ok(data) => data,
        Err(e) => {
            eprint!("Error deserialzing auth state {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error deserialzing auth state");
        }
    };

    let _auth_result = match webauthn.finish_passkey_authentication(&req, &deserialized_as) {
        Ok(result) => result,
        Err(e) => {
            eprint!("Error authenticating {:?}",e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error authenticating");
        }
    };

    let _del_auth_state = match db.auth_states.delete_by_username(username).await {
        Ok(response) => response,
        Err(e) => {
            eprint!("Error deleting the auth state of {:?} {:?}",username,e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Failed deleting the user auth state, try again after sometime");
        }
    };

    let uuid = match db.users.search_by_username(username).await.unwrap(){
        Some(user) => user.uuid,
        None => "1".to_string()
    };

    let jwt_token = match jwt.sign(uuid){
        Ok(token ) => token,
        Err(e) => {
            eprint!("Error generating the jwt token {:?} {:?}",username,e);
            return HttpResponse::InternalServerError().status(StatusCode::INTERNAL_SERVER_ERROR).json("Error generating the jwt token");
        }
    };

    let cookie = Cookie::build("auth_token", jwt_token).http_only(true).secure(false).same_site(SameSite::None).secure(true).path("/").max_age(Duration::seconds(3600)).finish();


    return HttpResponse::Ok().cookie(cookie).status(StatusCode::CREATED).json("User logged in!");
}

pub fn init(cnf: &mut ServiceConfig) -> () {
    cnf.service(start_registration).service(finish_registration).service(start_authentication).service(finish_authentication);
    ()
}
