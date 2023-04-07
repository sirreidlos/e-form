use std::str::FromStr;

use crate::routes::form::Form;
use bson::{doc, oid::ObjectId};
use mongodb::Database;
use rocket::{http::Status, response::status::Custom, State};
use serde_json::{json, Value};

pub mod form;
pub mod response;
pub mod template;
pub mod user;

pub async fn find_form_by_id(id: &str, db: &State<Database>) -> Result<Form, Custom<Value>> {
    let obj_id = object_id_from_string(id)?;
    match db
        .collection("forms")
        .find_one(doc! {"_id": obj_id}, None)
        .await
    {
        Ok(Some(form)) => Ok(form),
        Ok(None) => Err(Custom(
            Status::NotFound,
            json!({
                "message": "Form not found."
            }),
        )),
        Err(e) => {
            println!("{e} in find_form_by_id");
            Err(Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            ))
        }
    }
}

pub fn object_id_from_string(id: &str) -> Result<ObjectId, Custom<Value>> {
    match ObjectId::from_str(id) {
        Ok(id) => Ok(id),
        Err(_) => Err(Custom(
            Status::NotFound,
            json!({
                "message": "Form not found."
            }),
        )),
    }
}
