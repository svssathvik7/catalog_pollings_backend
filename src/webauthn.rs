use dotenv::dotenv;
use log::error;
use std::{env, error::Error};
use webauthn_rs::{prelude::Url, Webauthn, WebauthnBuilder};

pub fn config_webauthn() -> Result<Webauthn, Box<dyn Error>> {
    dotenv().ok();

    let rp_id = env::var("RP_ID").unwrap_or_else(|_| {
        error!("RP_ID env var not found");
        "localhost".to_string()
    });

    let rp_origin = env::var("RP_ORIGIN").unwrap_or_else(|_| {
        error!("RP_ORIGIN env var not found");
        "http://localhost:3000".to_string()
    });

    // Parse the URL, logging an error if it fails
    let rp_origin = Url::parse(&rp_origin).map_err(|err| {
        error!("Error parsing rp origin {}: {:?}", rp_origin, err);
        err
    })?;

    let builder = WebauthnBuilder::new(&rp_id, &rp_origin).map_err(|err| {
        error!("Failed building Webauthn: {:?}", err);
        err
    })?;

    let builder = builder.rp_name("Sathvik Polling Technologies!");
    let webauthn = builder.build().map_err(|err| {
        error!("Invalid configuration: {:?}", err);
        err
    })?;

    Ok(webauthn)
}
