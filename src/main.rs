#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod auth;
mod routes;

use mongodb::Client;
use mongodb::Database;

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
    let figment = Config::figment();
    let database_uri: String = figment
        .extract_inner("database")
        .expect("database uri be provided in Rocket.toml");

    let mongo_db = match Client::with_uri_str(database_uri).await {
        Ok(client) => client.database("e-form"),
        Err(e) => panic!("{e}"),
    };

    rocket::build()
        .attach(CORS)
        .manage(channel::<routes::response::Response>(1024).0)
        .manage::<Database>(mongo_db)
        .manage::<Figment>(figment)
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
                routes::response::response_chart,
                routes::response::response_chart_as_anon,
                routes::response::delete_response,
            ],
        )
        .mount(
            "/",
            routes![
                routes::template::get_all_templates,
                routes::template::get_template,
            ],
        )
}
