use actix_web::{web::Json, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum Status {
    Ok,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response<T> {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> Response<T>
where
    T: Serialize,
{
    pub fn ok(result: T) -> HttpResponse {
        let response = Json(Response {
            status: Status::Ok,
            result: Some(result),
            error: None,
        });
        HttpResponse::Ok().json(response)
    }
    pub fn error(error: &str) -> HttpResponse {
        let response: Json<Response<()>> = Json(Response {
            status: Status::Error,
            error: Some(error.to_string()),
            result: None,
        });
        HttpResponse::InternalServerError().json(response)
    }
}
