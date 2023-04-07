use bson::serde_helpers::{
    deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id,
};

use mongodb::bson::doc;

use mongodb::Database;
use rocket::{http::Status, response::status::Custom, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct Template {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    pub _id: String,
    pub title: String,
    pub description: String,
    pub questions: Vec<Question>,
    pub thumbnail_string: Option<String>,
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

pub async fn find_template_by_id(
    id: &str,
    db: &State<Database>,
) -> Result<Template, Custom<Value>> {
    let obj_id = crate::routes::object_id_from_string(id)?;
    match db
        .collection("templates")
        .find_one(doc! {"_id": obj_id}, None)
        .await
    {
        Ok(Some(form)) => Ok(form),
        Ok(None) => Err(Custom(
            Status::NotFound,
            json!({
                "message": "Template not found."
            }),
        )),
        Err(e) => {
            println!("{e} in find_template_by_id");
            Err(Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            ))
        }
    }
}

#[get("/templates")]
pub async fn get_all_templates(db: &State<Database>) -> Custom<Value> {
    let mut cursor = match db
        .collection::<Template>("templates")
        .find(doc! {}, None)
        .await
    {
        Ok(cursor) => cursor,
        Err(e) => {
            eprintln!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            );
        }
    };

    let mut templates = vec![];

    while cursor.advance().await.unwrap() {
        let current = cursor.deserialize_current().unwrap();
        templates.push(json!({
            "_id": current._id,
            "title": current.title,
            "thumbnail_string": current.thumbnail_string
        }))
    }

    Custom(Status::Ok, json!(templates))
}

#[get("/template/<id>")]
pub async fn get_template(id: String, db: &State<Database>) -> Custom<Value> {
    let template: Template = match find_template_by_id(&id, db).await {
        Ok(template) => template,
        Err(e) => {
            return e;
        }
    };

    Custom(
        Status::Ok,
        json!({
            "_id": template._id,
            "title": template.title,
            "description": template.description,
            "questions": template.questions,
        }),
    )
}
