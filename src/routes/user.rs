use mongodb::bson::doc;
use mongodb::{Client, Database};
use regex::Regex;
use rocket::{
    http::Status,
    response::status::Custom,
    serde::json::{Json, Value},
    State,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha512};
use sqlx::MySqlPool;

lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();
}

#[derive(FromForm, Deserialize, Debug)]
pub struct RegisterData {
    username: String,
    email: String,
    password: String,
}

#[derive(FromForm, Deserialize)]
pub struct LoginData {
    email: String,
    password: String,
}

struct User {
    id: String,
    email: String,
    username: String,
    password: String,
    created_at: sqlx::types::time::OffsetDateTime,
}

fn hash_password(password: &String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(password);
    format!("{:X}", hasher.finalize())
}

fn validate_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)
}

#[post("/register", format = "json", data = "<data>")]
pub async fn register(data: Json<RegisterData>, db: &State<Database>) -> Custom<Value> {
    if !validate_email(&data.email) {
        return Custom(
            Status::UnprocessableEntity,
            json!({"message": "Malformed email format"}),
        );
    }

    let db = db.inner().clone();
    match db
        .collection("users")
        .find_one(doc! {"email": &data.email}, None)
        .await
    {
        Ok(res) => {
            if res != None {
                return Custom(
                    Status::Conflict,
                    json!( {
                        "message": "An existing account has been made using this email.",
                    }),
                );
            }
        }
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!( {
                    "message": "An error has occurred.",
                }),
            );
        }
    }

    // match sqlx::query("SELECT * FROM `users` WHERE `email` = ?")
    //     .bind(&data.email)
    //     .fetch_one(db.inner())
    //     .await
    // {
    //     Ok(_) => {
    //         return Custom(
    //             Status::Conflict,
    //             json!( {
    //                 "message": "An existing account has been made using this email.",
    //             }),
    //         )
    //     }
    //     Err(e) => {
    //         if !matches!(e, sqlx::Error::RowNotFound) {
    //             println!("{e}");
    //             return Custom(
    //                 Status::InternalServerError,
    //                 json!( {
    //                     "message": "An error has occurred.",
    //                 }),
    //             );
    //         }
    //     }
    // }

    let hash = hash_password(&data.password);

    let id = uuid::Uuid::new_v4().to_string();

    use crate::auth::generate_jwt;
    let token = match generate_jwt(&id) {
        Ok(token) => token,
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An error has occurred.",
                }),
            );
        }
    };

    match sqlx::query("INSERT INTO `users` (id, email, username, password) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(&data.email)
        .bind(&data.username)
        .bind(hash)
        .execute(db.inner())
        .await
    {
        Ok(_) => Custom(
            Status::Created,
            json!({
                "message": "Success! Account has been created.",
                "token": token,
            }),
        ),
        Err(e) => {
            println!("{e}");

            Custom(
                Status::InternalServerError,
                json!({
                    "message": "Failed! An error has occurred, registration has failed.",
                }),
            )
        }
    }
}

#[post("/login", format = "json", data = "<data>")]
pub async fn login(data: Json<LoginData>, db: &State<MySqlPool>) -> Custom<Value> {
    if !validate_email(&data.email) {
        return Custom(
            Status::UnprocessableEntity,
            json!({
                "message": "Malformed email format.",
            }),
        );
    }

    // let query = ;
    let user = match sqlx::query_as!(User, "SELECT * FROM `users` WHERE email = ?", &data.email)
        .fetch_one(db.inner())
        .await
    {
        Ok(data) => data,
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::NotFound,
                json!({
                    "message": "Account not found.",
                }),
            );
        }
    };

    // let user = user.unwrap();

    let hash = hash_password(&data.password);
    if hash != user.password {
        return Custom(
            Status::Unauthorized,
            json!({
                "message": "Password inputted is incorrect.",
            }),
        );
    }

    use crate::auth::generate_jwt;
    let session_token: String = match generate_jwt(&user.id) {
        Ok(token) => token,
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            );
        }
    };

    Custom(
        Status::Ok,
        json!({
            "message": "Success! Account has been logged in.",
            "token": session_token,
        }),
    )
}
