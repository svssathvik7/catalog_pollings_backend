use actix_web::{dev::Path, web::{Data, ServiceConfig}, Responder};
use mongodb::Database;
use webauthn_rs::Webauthn;


// pub async fn registration_start(db: Data<Database>, username: Path<String>, webauthn: Data<Webauthn>) -> impl Responder{
    
// }

pub fn init(cnf: &mut ServiceConfig) -> (){
    ()
}