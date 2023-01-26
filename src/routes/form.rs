use std::str::FromStr;

use bson::{
    serde_helpers::{deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id},
    Document,
};

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use rocket::{http::Status, response::status::Custom, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::Auth;

#[derive(Serialize, Deserialize, Debug)]
struct Form {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    pub _id: String,
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    pub owner: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<Question>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Question {
    pub number: u32,
    pub text: String,
    pub kind: QuestionType,
    pub options: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
enum QuestionType {
    TextAnswer,
    MultipleChoice,
    Checkboxes,
    Dropdown,
    Date,
    Time,
}

#[get("/form/<id>")]
pub async fn get_form(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
    let form: Form = match db
        .collection("forms")
        .find_one(
            doc! {
                "_id": match ObjectId::from_str(&id) {
                    Ok(id) => id,
                    Err(e) => {
                        println!("{:?} | {}", e, e);
                        return Custom(
                        Status::NotFound,
                        json!({
                            "message": "Form not found."
                        })
                    );}
                }
            },
            None,
        )
        .await
    {
        Ok(Some(form)) => form,
        Ok(None) => {
            return Custom(
                Status::NotFound,
                json!({
                    "message": "Form not found."
                }),
            )
        }
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            );
        }
    };

    Custom(
        Status::Ok,
        json!({
            "_id": form._id,
            "owner": form.owner,
            "title": form.title,
            "description": form.description,
            "questions": form.questions,
            "created_at": form.created_at.to_rfc3339(),
        }),
    )
}

#[get("/form/edit/<id>")]
pub async fn edit_form(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
    let form: Form = match db
        .collection::<Form>("forms")
        .find_one(
            doc! {
                "_id": ObjectId::from_str(&id).unwrap(),
            },
            None,
        )
        .await
    {
        Ok(Some(form)) => {
            if form.owner != user_id.0 {
                return Custom(
                    Status::Unauthorized,
                    json!({
                        "message": "You are not the owner of this form."
                    }),
                );
            }

            form
        }
        Ok(None) => {
            return Custom(
                Status::NotFound,
                json!({
                    "message": "Form not found."
                }),
            )
        }
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            );
        }
    };

    Custom(
        Status::Ok,
        json!({
            "_id": form._id,
            "owner": form.owner,
            "title": form.title,
            "description": form.description,
            "questions": form.questions,
            "created_at": form.created_at.to_rfc3339(),
        }),
    )
}

// #[get("/form/<id>", rank = 2)]
// pub async fn get_form_as_anon(id: String, db: &State<Database>) {}

#[post("/form")]
pub async fn make_form(user_id: Auth, db: &State<Database>) {}

#[put("/form/<id>")]
pub async fn update_form(id: String, user_id: Auth, db: &State<Database>) {}

#[delete("/form/<id>")]
pub async fn delete_form(id: String, user_id: Auth, db: &State<Database>) {}

#[post("/form/<id>")]
pub async fn submit_response(id: String, user_id: Auth, db: &State<Database>) {}
