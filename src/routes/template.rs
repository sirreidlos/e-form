use std::str::FromStr;

use bson::{
    serde_helpers::{deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id},
    Document,
};

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use rocket::{http::Status, response::status::Custom, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::Auth;
use crate::routes::find_form_by_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct Form {
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
    pub state: FormState,
    pub questions: Vec<Question>,
    pub thumbnail_string: Option<String>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    pub number: u32,
    pub text: String,
    pub kind: QuestionType,
    pub options: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QuestionType {
    TextAnswer,
    MultipleChoice,
    Checkboxes,
    Dropdown,
    Date,
    Time,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum FormState {
    Private,
    Public,
    Anonymous,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormData {
    pub title: String,
    pub description: String,
    pub state: FormState,
    pub thumbnail_string: Option<String>,
    pub questions: Vec<Question>,
}

#[get("/form/<id>")]
pub async fn get_form(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
    let form: Form = match find_form_by_id(&id, db).await {
        Ok(form) => form,
        Err(e) => {
            return e;
        }
    };

    if form.state == FormState::Private && form.owner != user_id.0 {
        return Custom(
            Status::Forbidden,
            json!({
                "message": "This form is private."
            }),
        );
    }

    let status = {
        if form.owner == user_id.0 {
            String::from("owner")
        } else {
            String::from("responder")
        }
    };

    Custom(
        Status::Ok,
        json!({
            "_id": form._id,
            "owner": form.owner,
            "title": form.title,
            "description": form.description,
            "user_status": status,
            "state": form.state,
            "questions": form.questions,
            "created_at": form.created_at.to_rfc3339(),
        }),
    )
}

#[get("/template/<id>")]
pub async fn get_template(id: String, db: &State<Database>) -> Custom<Value> {
    let form: Form = match find_form_by_id(&id, db).await {
        Ok(form) => form,
        Err(e) => {
            return e;
        }
    };

    if form.state != FormState::Anonymous {
        return Custom(
            Status::Unauthorized,
            json!({
                "message": "This form requires you to be authenticated. Log in."
            }),
        );
    }

    Custom(
        Status::Ok,
        json!({
            "_id": form._id,
            "owner": form.owner,
            "title": form.title,
            "description": form.description,
            "user_status": String::from("responder"),
            "state": form.state,
            "questions": form.questions,
            "created_at": form.created_at.to_rfc3339(),
        }),
    )
}
