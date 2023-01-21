#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod auth;
mod routes;

use std::collections::HashMap;
use std::sync::Mutex;

use mongodb::Database;
use rocket::fairing::{AdHoc, Fairing, Info, Kind};
use rocket::fs::{relative, FileServer};

use rocket::http::{ContentType, Header, Method, Status};
use rocket::request::Request;

use rocket::tokio::sync::broadcast::channel;
use rocket::Response;

use serde_json::{json, Value};
use sqlx::MySqlPool;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "content-type"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));

        if request.method() == Method::Options {
            let body = "";
            response.set_header(ContentType::Plain);
            response.set_sized_body(body.len(), std::io::Cursor::new(body));
            response.set_status(Status::Ok);
        }
    }
}

use auth::generate_jwt;
#[get("/<user_id>")]
fn test_token(user_id: &str) -> Option<Value> {
    // Some(json!({ "token": generate_jwt(user_id) }))
    match generate_jwt(user_id) {
        Ok(token) => Some(json!({ "token": token })),
        Err(_) => None,
    }
}

use auth::UserID;
#[get("/attempt")]
fn attempt(user_id: UserID) -> Option<Value> {
    Some(json!({ "token": user_id.0 }))
}

use mongodb::{bson::doc, bson::oid::ObjectId, options::ClientOptions, Client};
#[launch]
async fn rocket() -> _ {
    // let db_url = "mysql://root@localhost/e_form";
    // let pool = match MySqlPool::connect(db_url).await {
    //     Ok(pool) => pool,
    //     Err(error) => panic!("Failed to connect to database: {error}"),
    // };

    let mongo_db = match Client::with_uri_str("mongodb://localhost:27017/e-form").await {
        Ok(client) => client.database("e-form"),
        Err(e) => panic!("{e}"),
    };

    rocket::build()
        .attach(CORS)
        // .manage::<MySqlPool>(pool)
        .manage::<Database>(mongo_db)
        .mount("/", routes![test_token, attempt])
        .mount("/", routes![routes::user::login, routes::user::register])
        .mount("/", FileServer::from(relative!("static")))
}
