use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{
    cookie::{time::Duration, Cookie, SameSite},
    http::StatusCode,
    web::{Data, Json, Path, ServiceConfig},
    HttpResponse, Responder,
};
use log::error;
use mongodb::bson::oid::ObjectId;
use webauthn_rs::{
    prelude::{
        Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
        RegisterPublicKeyCredential, Uuid,
    },
    Webauthn,
};

use crate::{
    db::{auth_state_repo::AuthState, reg_state_repo::RegState, users_repo::User, DB},
    utils::{json_responder::Response, jwt::JWT},
};

#[actix_web::post("/register/start")]
pub async fn start_registration(
    db: Data<Arc<Mutex<DB>>>,
    webauthn: Data<Webauthn>,
    Json(req): Json<HashMap<String, String>>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let username = match req.get("username") {
        Some(username) => username,
        None => {
            return Response::<()>::error("No username found", StatusCode::BAD_REQUEST);
        }
    };
    let users_match = db.users.search_by_username(username).await;

    let uuid = match users_match {
        Ok(Some(_user)) => {
            return Response::<()>::error("Username already exists!", StatusCode::BAD_REQUEST);
        }
        Ok(None) => Uuid::new_v4(),
        Err(e) => {
            error!("Error searching user with username {} : {}", username, e);
            return Response::<()>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let (ccr, reg_state) = match webauthn.start_passkey_registration(uuid, username, username, None)
    {
        Ok(data) => data,
        Err(e) => {
            error!("Error creating challange and reg state {}", e);
            return Response::<()>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    // serialize the reg state
    let reg_state = match serde_json::to_value(&reg_state) {
        Ok(value) => value,
        Err(e) => {
            error!("Error serializing reg_state {}", e);
            return Response::<()>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let new_user_reg_state = RegState {
        username: username.to_string(),
        uuid: uuid.to_string(),
        reg_state,
    };

    let result = match db.reg_states.insert(new_user_reg_state).await {
        Ok(_success) => Response::ok(ccr, StatusCode::OK),
        Err(e) => {
            error!("Error storing reg state to db {}", e);
            Response::<()>::error("Something went wrong", StatusCode::INTERNAL_SERVER_ERROR)
        }
    };
    result
}

#[actix_web::post("/register/finish/{username}")]
pub async fn finish_registration(
    db: Data<Arc<Mutex<DB>>>,
    webauthn: Data<Webauthn>,
    username: Path<String>,
    request: Json<RegisterPublicKeyCredential>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let username = username.as_str();
    let _does_reg_state_exist = match db.reg_states.is_exists(username).await {
        Ok(data) => {
            if data {
                data
            } else {
                return Response::<String>::error(
                    "No registration init found!",
                    StatusCode::BAD_REQUEST,
                );
            }
        }
        Err(e) => {
            error!("Error registering {}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let reg_state_match = db.reg_states.find_by_username(username).await;
    let reg_state_match = if let Ok(Some(doc_match)) = reg_state_match {
        doc_match.reg_state
    } else {
        eprintln!("Error at register finish!");
        return Response::<String>::error(
            "Error registering user!",
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    };
    let reg_state_match = match serde_json::to_value(reg_state_match) {
        Ok(serialized_reg_state) => serialized_reg_state,
        Err(e) => {
            error!("Error registering state {:?}", e);
            return Response::<()>::error(
                "Error registering user!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let deserialize_reg_state: PasskeyRegistration = match serde_json::from_value(reg_state_match) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed deserializing registration state {:?}", e);
            return Response::<String>::error(
                "Error registering user!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let pass_key = match webauthn.finish_passkey_registration(&request, &deserialize_reg_state) {
        Ok(key) => key,
        Err(e) => {
            error!("Failed generating the passkey {:?}", e);
            return Response::<String>::error(
                "Failed generating passkey",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    // serialize the key
    let sk = match serde_json::to_value(pass_key) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed serializing the key {}", e);
            return Response::<String>::error(
                "Failed generating passkey!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let new_user = User {
        id: Some(ObjectId::new()),
        username: username.to_string(),
        uuid: Uuid::new_v4().to_string(),
        sk,
    };

    let result = match db.users.insert(new_user).await {
        Ok(_inserted_user) => Response::ok(username, StatusCode::CREATED),
        Err(e) => {
            error!("Failed registering the user! {:?}", e);
            return Response::<String>::error(
                "Failed registering user!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let _del_reg_state = match db.reg_states.delete_by_username(username).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error deleting the reg state of {} {}", username, e);
            return Response::<String>::error(
                "Error creating user!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    result
}

#[actix_web::post("/login/start")]
pub async fn start_authentication(
    db: Data<Arc<Mutex<DB>>>,
    webauthn: Data<Webauthn>,
    Json(req): Json<HashMap<String, String>>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let username = match req.get("username") {
        Some(username) => username,
        None => {
            return Response::<String>::error("No username found!", StatusCode::BAD_REQUEST);
        }
    };
    let _does_user_exist = match db.users.is_exists(username).await {
        Ok(boolean_response) => {
            if boolean_response {
                boolean_response
            } else {
                return Response::<String>::error("No user found!", StatusCode::BAD_REQUEST);
            }
        }
        Err(e) => {
            error!("Error finding user! {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let user_sk = db.users.search_by_username(username).await;
    let user_sk = if let Ok(Some(user)) = user_sk {
        user.sk
    } else {
        eprintln!("Error fetching user!");
        return Response::<String>::error(
            "Error authenticating user!",
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    };

    // for now just a single passkey
    let sk: Vec<Passkey> = match serde_json::from_value(user_sk) {
        Ok(key) => vec![key],
        Err(e) => {
            error!("Error deserializing sk {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let (rcr, auth_state) = match webauthn.start_passkey_authentication(&sk) {
        Ok(data) => data,
        Err(e) => {
            error!("Error generating auth challange {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let serial_auth_state = match serde_json::to_value(auth_state) {
        Ok(serial_auth_state) => serial_auth_state,
        Err(e) => {
            error!("Error serialzing auth state {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let auth_state_entry = AuthState {
        auth_state: serial_auth_state,
        username: username.to_string(),
    };

    let _result = match db.auth_states.insert(auth_state_entry).await {
        Ok(inserted) => inserted,
        Err(e) => {
            error!("Error writing auth state to db {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    return Response::ok(rcr, StatusCode::OK);
}

#[actix_web::post("/login/finish/{username}")]
pub async fn finish_authentication(
    username: Path<String>,
    db: Data<Arc<Mutex<DB>>>,
    webauthn: Data<Webauthn>,
    req: Json<PublicKeyCredential>,
    jwt: Data<JWT>,
) -> impl Responder {
    let db = db.lock().unwrap();
    let username = username.as_str();
    let _does_auth_state_exist = match db.auth_states.is_exists(username).await {
        Ok(data) => {
            if data {
                data
            } else {
                return Response::<String>::error("No user found!", StatusCode::BAD_REQUEST);
            }
        }
        Err(e) => {
            error!("Error authenticating {:?}", e);
            return Response::<String>::error("No user found!", StatusCode::BAD_REQUEST);
        }
    };

    let user_auth_state = match db.auth_states.find_by_username(username).await {
        Ok(Some(data)) => data.auth_state,
        Ok(None) => {
            return Response::<String>::error("No user found!", StatusCode::BAD_REQUEST);
        }
        Err(e) => {
            error!("Error fetching auth state {:?}", e);
            return HttpResponse::InternalServerError()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json("Error fetching auth state");
        }
    };

    let deserialized_as: PasskeyAuthentication = match serde_json::from_value(user_auth_state) {
        Ok(data) => data,
        Err(e) => {
            error!("Error deserialzing auth state {}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let _auth_result = match webauthn.finish_passkey_authentication(&req, &deserialized_as) {
        Ok(result) => result,
        Err(e) => {
            error!("Error authenticating {:?}", e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let _del_auth_state = match db.auth_states.delete_by_username(username).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error deleting the auth state of {:?} {:?}", username, e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let uuid = match db.users.search_by_username(username).await.unwrap() {
        Some(user) => user.uuid,
        None => "1".to_string(),
    };

    let jwt_token = match jwt.sign(uuid) {
        Ok(token) => token,
        Err(e) => {
            error!("Error generating the jwt token {} {}", username, e);
            return Response::<String>::error(
                "Something went wrong!",
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };

    let cookie = Cookie::build("auth_token", jwt_token)
        .http_only(true)
        .same_site(SameSite::None)
        .secure(true)
        .path("/")
        .max_age(Duration::hours(1))
        .finish();

    return HttpResponse::Ok()
        .cookie(cookie)
        .status(StatusCode::CREATED)
        .json("User logged in!");
}

#[actix_web::get("/logout")]
pub async fn logout_user() -> impl Responder {
    let cookie = Cookie::build("auth_token", "")
        .http_only(true)
        .same_site(SameSite::None)
        .secure(true)
        .path("/")
        .max_age(Duration::days(-1))
        .finish();
    return HttpResponse::Ok()
        .cookie(cookie)
        .status(StatusCode::CREATED)
        .json("User logged out!");
}

pub fn init(cnf: &mut ServiceConfig) -> () {
    cnf.service(start_registration)
        .service(finish_registration)
        .service(start_authentication)
        .service(finish_authentication)
        .service(logout_user);
    ()
}
