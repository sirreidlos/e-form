#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod auth;
mod routes;

use std::time;
use std::time::Duration;

use mongodb::Client;
use mongodb::Database;
use rocket::fairing::AdHoc;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::figment::Figment;
use rocket::http::{ContentType, Header, Method, Status};
use rocket::request::Request;
use rocket::tokio::sync::broadcast::channel;
use rocket::{Config, Response};

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
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));

        if request.method() == Method::Options {
            let body = "";
            response.set_header(ContentType::Plain);
            response.set_sized_body(body.len(), std::io::Cursor::new(body));
            response.set_status(Status::Ok);
        }
    }
}

#[launch]
pub async fn rocket() -> _ {
    let mongo_db = match Client::with_uri_str("mongodb://localhost:27017/e-form").await {
        Ok(client) => client.database("e-form"),
        Err(e) => panic!("{e}"),
    };

    let config = Config::figment();

    rocket::build()
        .attach(CORS)
        .manage(channel::<routes::response::Response>(1024).0)
        .manage::<Database>(mongo_db)
        .manage::<Figment>(config)
        .mount("/", routes![routes::user::login, routes::user::register])
        .mount(
            "/",
            routes![
                routes::form::get_all_owned_forms,
                routes::form::get_form,
                routes::form::get_form_as_anon,
                routes::form::post_form,
                routes::form::post_form_as_anon,
                routes::form::put_form,
                routes::form::put_form_as_anon,
                routes::form::delete_form,
                routes::form::delete_form_as_anon,
            ],
        )
        .mount(
            "/",
            routes![
                routes::response::get_response,
                routes::response::post_response,
                routes::response::response_stream,
                routes::response::response_stream_as_anon,
                routes::response::delete_response,
            ],
        )
    // .attach(AdHoc::on_liftoff("Shutdown", |rocket| {
    //     Box::pin(async move {
    //         let shutdown = rocket.shutdown();
    //         rocket::tokio::spawn(async move {
    //             rocket::tokio::time::sleep(rocket::tokio::time::Duration::from_secs(15)).await;

    //             shutdown.notify();
    //         });
    //     })
    // }))

    // .mount("/s1", FileServer::from(relative!("static")))
    // .mount("/s2", FileServer::from(relative!("static2")))
}
