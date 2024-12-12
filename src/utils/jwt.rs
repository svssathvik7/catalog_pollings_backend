use std::env;

use chrono::{Duration, Utc};
use dotenv::dotenv;
use jsonwebtoken::{encode, EncodingKey, Header};
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

    pub fn sign(&self, uuid: String) -> Result<String, jsonwebtoken::errors::Error>{
        let exp = Utc::now().checked_add_signed(Duration::hours(1)).ok_or("Failed to compile exp time").unwrap().timestamp() as usize;
        let claims = Claims::init(uuid, exp);
        let header = &Header::default();
        let encoded_secret = EncodingKey::from_secret(self.secret.as_bytes());
        let token = encode(header, &claims, &encoded_secret);
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign() {
        let jwt = JWT::init();
        let token = jwt.sign("test-uuid".to_string());
        assert!(token.is_ok());
        println!("Generated Token: {:?}", token.unwrap());
    }
}
