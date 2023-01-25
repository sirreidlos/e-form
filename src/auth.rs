use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};

const SECRET: &[u8] = b"secret";

#[derive(Debug)]
pub struct Auth(pub String);

#[async_trait]
impl<'a> FromRequest<'a> for Auth {
    type Error = ();

    async fn from_request(request: &'a Request<'_>) -> request::Outcome<Auth, ()> {
        let headers = request.headers();
        let auth = headers.get_one("Authorization");

        match auth {
            Some(bearer) => {
                let token = bearer.replace("Bearer ", "");
                match decode::<Claims>(
                    &token,
                    &DecodingKey::from_secret(SECRET),
                    &Validation::default(),
                ) {
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
struct Claims {
    pub exp: u64,
    pub sub: String,
}

pub fn generate_jwt(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let TWO_HOURS_IN_SECONDS = 2 * 60 * 60;

    let claims = Claims {
        exp: now + TWO_HOURS_IN_SECONDS,
        sub: user_id.to_string(),
    };

    let header = Header::default();

    encode(&header, &claims, &EncodingKey::from_secret(SECRET))
}
