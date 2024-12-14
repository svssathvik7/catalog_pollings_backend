use actix_web::{dev::{ServiceRequest, ServiceResponse}, http::StatusCode, middleware::Next, web::Data, HttpResponse};
use log::info;
use crate::utils::jwt::JWT;
use actix_web::body::BoxBody;

pub async fn authenticate_user(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    info!("Authentication middleware!");
    let jwt = req.app_data::<Data<JWT>>().expect("JWT not configured").clone();
    if let Some(cookie) = req.cookie("auth_token") {
        let token = cookie.value();

        if jwt.verify(token) {
            return next.call(req).await;
        } else {
            return Ok(req.into_response(HttpResponse::Unauthorized()
                .status(StatusCode::FORBIDDEN)
                .json("Invalid or expired token!")));
        }
    }

    return Ok(req.into_response(HttpResponse::BadRequest()
        .status(StatusCode::BAD_REQUEST)
        .body("Missing auth token cookie")));
}
