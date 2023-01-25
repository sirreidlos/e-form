use chrono::{DateTime, Utc};

use bson::serde_helpers::{
    deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id,
};
use serde::{Deserialize, Serialize};

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
