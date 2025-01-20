use std::env;

use chrono::{Duration, Utc};
use dotenv::dotenv;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Claims {
    pub uuid: String,
    pub exp: usize,
}

impl Claims {
    pub fn init(uuid: String, exp: usize) -> Self {
        dotenv().ok();
        Claims { uuid, exp }
    }
}

pub struct JWT {
    secret: String,
}

impl JWT {
    pub fn init() -> Self {
        dotenv().ok();
        let jwt_secret = env::var("JWT_SECRET").expect("Set jwt secret to sign!");
        JWT { secret: jwt_secret }
    }

    pub fn sign(&self, uuid: String) -> Result<String, jsonwebtoken::errors::Error> {
        let exp = Utc::now()
            .checked_add_signed(Duration::hours(1))
            .ok_or("Failed to compile exp time")
            .unwrap()
            .timestamp() as usize;
        let claims = Claims::init(uuid, exp);
        let header = &Header::default();
        let encoded_secret = EncodingKey::from_secret(self.secret.as_bytes());
        let token = encode(header, &claims, &encoded_secret);
        token
    }

    pub fn verify(&self, token: &str) -> bool {
        let mut validations = Validation::new(Algorithm::HS256);
        validations.validate_exp = true;
        if decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validations,
        )
        .is_ok()
        {
            return true;
        }

        false
    }

    pub fn decode(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validations = Validation::new(Algorithm::HS256);
        validations.validate_exp = true;
        let decoded_token = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validations,
        )?;
        Ok(decoded_token.claims)
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
