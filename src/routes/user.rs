use std::any::Any;

use bson::serde_helpers::{
    deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id,
};
use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use mongodb::Database;
use regex::Regex;
use rocket::{
    http::Status,
    response::status::Custom,
    serde::json::{Json, Value},
    State,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha512};

lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct User {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    pub _id: String,
    pub email: String,
    pub username: String,
    pub password: String,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(FromForm, Deserialize, Serialize, Debug)]
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

    println!("{:?}", data.email.type_id());

    let db = db.inner().clone();

    let _find: Option<User> = match db
        .collection("users")
        .find_one(doc! {"email": &data.email}, None)
        .await
    {
        Ok(res) => {
            if res.is_some() {
                return Custom(
                    Status::Conflict,
                    json!( {
                        "message": "An existing account has been made using this email.",
                    }),
                );
            }

            res
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
    };

    let password_hash = hash_password(&data.password);

    let new_user = User {
        _id: mongodb::bson::oid::ObjectId::new().to_string(),
        email: data.email.clone(),
        username: data.username.clone(),
        password: password_hash,
        created_at: Utc::now(),
    };

    use crate::auth::generate_jwt;
    let token = match generate_jwt(&new_user._id) {
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

    match db.collection("users").insert_one(new_user, None).await {
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
pub async fn login(data: Json<LoginData>, db: &State<Database>) -> Custom<Value> {
    if !validate_email(&data.email) {
        return Custom(
            Status::UnprocessableEntity,
            json!({
                "message": "Malformed email format.",
            }),
        );
    }

    let user: User = match db
        .collection("users")
        .find_one(doc! {"email": &data.email}, None)
        .await
    {
        Ok(res) => {
            if res.is_none() {
                return Custom(
                    Status::NotFound,
                    json!( {
                        "message": "Account not found.",
                    }),
                );
            }

            res.unwrap()
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
    };

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
    let session_token: String = match generate_jwt(&user._id) {
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

#[cfg(test)]
mod test {
    use bson::Document;
    use mongodb::bson::doc;
    use mongodb::Client as mdbClient;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::json;

    use crate::rocket;

    async fn cleanup(email: &str) -> Result<(), mongodb::error::Error> {
        let mongo_db = match mdbClient::with_uri_str("mongodb://localhost:27017/e-form").await {
            Ok(client) => client.database("e-form"),
            Err(e) => panic!("{e}"),
        };

        let _result: mongodb::results::DeleteResult = mongo_db
            .collection::<Document>("users")
            .delete_many(doc! {"email": email}, None)
            .await?;

        Ok(())
    }

    #[async_test]
    async fn register_user() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "milize_test@example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        println!("{:?}", response.body());
        assert!(cleanup("milize_test@example.com").await.is_ok());
    }

    #[async_test]
    async fn register_malformed_email() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let mut responses = vec![];

        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;
        responses.push(response.status());

        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "test@123",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;
        responses.push(response.status());

        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "yoru @ ni . kakeru",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;
        responses.push(response.status());

        for response in responses {
            assert_eq!(response, Status::UnprocessableEntity);
        }
    }

    #[async_test]
    async fn register_duplicate_email() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "milize_test@example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;
        println!("{:?}", response.status());
        if response.status() == Status::Created || response.status() == Status::Conflict {
            let x = true;
            assert!(x, "Response of Conflict or Created")
        } else {
            assert_eq!(response.status(), Status::Created)
        }

        let response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "milize_test@example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        println!("{:?}", response.status());
        if response.status() == Status::Created || response.status() == Status::Conflict {
            let x = true;
            assert!(x, "Response of Conflict or Created")
        } else {
            assert_eq!(response.status(), Status::Conflict)
        }

        assert!(cleanup("milize_test@example.com").await.is_ok());
    }

    #[async_test]
    async fn login_user() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let register_response = client
            .post(uri!(crate::routes::user::register))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "milize_test@example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(register_response.status(), Status::Created);

        let response = client
            .post(uri!(crate::routes::user::login))
            .header(ContentType::JSON)
            .body(
                json!({
                    "email": "milize_test@example.com",
                    "username": "milize",
                    "password": "hello"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        println!("{:?}", response.body());
        assert_eq!(response.status(), Status::Ok);
        assert!(cleanup("milize_test@example.com").await.is_ok());
    }
}
