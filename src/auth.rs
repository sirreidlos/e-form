use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rocket::figment::Figment;
use rocket::http::Status;
use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};
use rocket::Config;

// const SECRET: &[u8] = b"secret";

lazy_static! {
    static ref SECRET_KEY: String = {
        let config = Config::figment();

        config
            .extract_inner("secret_key")
            .expect("secret key expected")
    };
}

#[derive(Debug)]
pub struct Auth(pub String);

#[async_trait]
impl<'a> FromRequest<'a> for Auth {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> request::Outcome<Auth, ()> {
        let figment = request.rocket().state::<Figment>().unwrap();

        let headers = request.headers();
        let auth = headers.get_one("Authorization");

        match auth {
            Some(bearer) => {
                let token = bearer.replace("Bearer ", "");
                match decode_jwt(&token, &DecodingKey::from_secret(SECRET_KEY.as_bytes())) {
                    Ok(data) => Outcome::Success(Auth(data.claims.sub)),
                    Err(_) => Outcome::Failure((Status::Unauthorized, ())),
                }
            }
            None => Outcome::Failure((Status::Unauthorized, ())),
        }
    }
}

use jsonwebtoken::encode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: u64,
    pub sub: String,
}

const TWO_HOURS_IN_SECONDS: u64 = 2 * 60 * 60;

pub fn decode_jwt(
    token: &str,
    key: &DecodingKey,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let result = decode::<Claims>(token, key, &Validation::default());
    println!("{result:?}");
    result
}

pub fn generate_jwt(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = Claims {
        exp: now + TWO_HOURS_IN_SECONDS,
        sub: user_id.to_string(),
    };

    let header = Header::default();

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(SECRET_KEY.as_bytes()),
    )
}
