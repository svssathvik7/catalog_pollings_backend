use actix_web::{
    dev::Path,
    web::{Data, ServiceConfig},
    Responder,
};
use mongodb::Database;
use webauthn_rs::Webauthn;

use crate::db::DB;

// pub async fn registration_start(db: Data<DB>, username: Path<String>, webauthn: Data<Webauthn>) -> impl Responder{
//     let users_collection = db.users.
// }

pub fn init(cnf: &mut ServiceConfig) -> () {
    ()
}
