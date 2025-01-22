use log::error;
use std::{error::Error, sync::Arc};
use webauthn_rs::{prelude::Url, Webauthn, WebauthnBuilder};

use crate::config::app_config::AppConfig;

pub fn config_webauthn(app_config: Arc<AppConfig>) -> Result<Webauthn, Box<dyn Error>> {
    let rp_id = &app_config.rp_id;

    let rp_origin = &app_config.rp_origin;
    println!("{} {}", rp_id, rp_origin);

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
