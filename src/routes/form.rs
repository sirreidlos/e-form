use bson::serde_helpers::{
    deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id,
};

use chrono::{DateTime, Utc};
use mongodb::Database;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

#[derive(Serialize, Deserialize)]
struct Form {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    _id: String,
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    owner: String,
    title: String,
    description: String,
    questions: Vec<Question>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct Question {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    _id: String,
    text: String,
    kind: QuestionType,
    input: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
enum QuestionType {
    TextAnswer,
    MultipleChoice,
    Checkboxes,
    Dropdown,
    Date,
    Time,
}

#[get("/forms/<id>")]
pub async fn get_forms(id: String, user_id: Auth, db: &State<Database>) {}
