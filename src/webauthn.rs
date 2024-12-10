use std::error::Error;

use actix_web::web::Data;
use webauthn_rs::{prelude::Url, Webauthn, WebauthnBuilder};

pub fn config_webauthn() -> Result<Data<Webauthn>, Box<dyn Error>> {
    let rp_id = "localhost";
    let rp_origin = Url::parse("localhost:5000").expect("Invalid URL");
    let builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Failed to build webauthn!");

    let builder = builder.rp_name("Sathvik Polling Technologies!");

    let webauthn = Data::new(builder.build().expect("Invalid configuration!"));

    Ok(webauthn)
}
