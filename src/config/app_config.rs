use dotenv::dotenv;
use log::error;
use std::env;

pub struct AppConfig {
    pub token_secret: String,
    pub db_url: String,
    pub jwt_secret: String,
    pub rp_id: String,
    pub rp_origin: String,
    pub is_dev: bool,
    pub client_origin: String,
    pub server_addr: String,
}

impl AppConfig {
    pub fn init() -> Self {
        dotenv().ok();
        let is_dev = env::var("IS_DEV").map(|v| v == "true").unwrap_or_else(|_| {
            error!("is_dev var not found!");
            false
        });
        let token_secret = env::var("TOKEN_SECRET").unwrap_or_else(|_| {
            error!("token_secret var not found!");
            String::from("Catalog")
        });
        let db_url: String;
        let rp_id: String;
        let rp_origin: String;
        let client_origin: String;
        let server_addr: String;
        if !is_dev {
            db_url = env::var("PROD_DB_URL").expect("No DB url found!");
            rp_id = env::var("PROD_RP_ID").expect("No rp id found!");
            rp_origin = env::var("PROD_RP_ORIGIN").expect("No rp origin found!");
            client_origin = env::var("PROD_CLIENT_ORIGIN").expect("No client origin found!");
            server_addr = env::var("PROD_SERVER_ADDR").expect("No server origin found!");
        } else {
            db_url = env::var("DEV_DB_URL").expect("No DB url found!");
            rp_id = env::var("DEV_RP_ID").expect("No rp id found!");
            rp_origin = env::var("DEV_RP_ORIGIN").expect("No rp origin found!");
            client_origin = env::var("DEV_CLIENT_ORIGIN").expect("No client origin found!");
            server_addr = env::var("DEV_SERVER_ADDR").expect("No server origin found!");
        };
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            error!("jwt_secret var not set!");
            String::from("Garden")
        });
        Self {
            db_url,
            is_dev,
            jwt_secret,
            rp_id,
            rp_origin,
            token_secret,
            client_origin,
            server_addr,
        }
    }
}
