use std::env;

use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Deserialize,Serialize,Debug)]
pub struct Claims{
    pub uuid: String,
    pub exp: usize,
    // ensures no tampering & authenticity
    pub token_secret: String
}

impl Claims{
    pub fn init(uuid:String,exp:usize) -> Self{
        dotenv().ok();
        let token_secret = env::var("TOKEN_SECRET").expect("Required token secret key!");
        Claims{
            uuid,
            exp,
            token_secret
        }
    }
}

pub struct JWT{
    secret: String
}

impl JWT{
    pub fn init() -> Self{
        dotenv().ok();
        let jwt_secret = env::var("JWT_SECRET").expect("Set jwt secret to sign!");
        JWT{
            secret: jwt_secret
        }
    }
}