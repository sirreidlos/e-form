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

#[get("/form/<id>", rank = 2)]
pub async fn get_form_as_anon(id: String, db: &State<Database>) -> Custom<Value> {
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

#[post("/form", format = "json", data = "<data>")]
pub async fn post_form(user_id: Auth, data: Json<FormData>, db: &State<Database>) -> Custom<Value> {
    let mut questions = data.questions.clone();
    for (i, question) in questions.clone().iter().enumerate() {
        match question.kind {
            QuestionType::TextAnswer | QuestionType::Date | QuestionType::Time => {
                questions[i].options = None;
            }
            _ => (),
        }
    }

    let form = Form {
        _id: ObjectId::new().to_string(),
        owner: user_id.0,
        title: data.title.clone(),
        description: data.description.clone(),
        state: data.state.clone(),
        questions,
        created_at: Utc::now(),
    };

    match db.collection("forms").insert_one(form, None).await {
        Ok(_) => Custom(
            Status::Created,
            json!({
                "message": "Form created."
            }),
        ),
        Err(e) => {
            println!("{e}");
            Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            )
        }
    }
}

#[post("/form/<_>", rank = 2)]
pub async fn post_form_as_anon() -> Custom<Value> {
    Custom(
        Status::Unauthorized,
        json!({
            "message": "You are not authorized. Log in."
        }),
    )
}

#[put("/form/<id>", format = "json", data = "<data>")]
pub async fn put_form(
    id: String,
    data: Json<FormData>,
    user_id: Auth,
    db: &State<Database>,
) -> Custom<Value> {
    let form: Form = match find_form_by_id(&id, db).await {
        Ok(form) => form,
        Err(e) => {
            return e;
        }
    };

    if form.owner != user_id.0 {
        return Custom(
            Status::Forbidden,
            json!({
                "message": "You are not the owner of this form."
            }),
        );
    }

    let state = bson::to_bson(&data.state).unwrap();
    let questions = bson::to_bson(&data.questions).unwrap();

    match db
        .collection::<Document>("forms")
        .update_one(
            doc! {"_id": form._id},
            doc! {"$set": {
                    "title": data.title.clone(),
                    "description": data.description.clone(),
                    "state": state,
                    "questions": questions,
            }},
            None,
        )
        .await
    {
        Ok(result) => {
            println!("{result:?}");
            Custom(
                Status::Ok,
                json!({
                    "message": "Form updated."
                }),
            )
        }
        Err(e) => {
            println!("{e}");
            Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            )
        }
    }
}

#[put("/form/<_>", rank = 2)]
pub async fn put_form_as_anon() -> Custom<Value> {
    Custom(
        Status::Unauthorized,
        json!({
            "message": "You are not authorized. Log in."
        }),
    )
}

#[delete("/form/<id>")]
pub async fn delete_form(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
    let form: Form = match find_form_by_id(&id, db).await {
        Ok(form) => form,
        Err(e) => {
            return e;
        }
    };

    if form.owner != user_id.0 {
        return Custom(
            Status::Forbidden,
            json!({
                "message": "You are not the owner of this form."
            }),
        );
    }

    match db
        .collection::<Document>("forms")
        .delete_one(doc! {"_id": form._id}, None)
        .await
    {
        Ok(_) => Custom(Status::Ok, json!({"message": "Form deleted."})),
        Err(e) => {
            println!("{e} in delete_one");
            Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occured."
                }),
            )
        }
    }
}

#[delete("/form/<_>", rank = 2)]
pub async fn delete_form_as_anon() -> Custom<Value> {
    Custom(
        Status::Unauthorized,
        json!({
            "message": "You are not authorized. Log in."
        }),
    )
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use bson::oid::ObjectId;
    use bson::Document;
    use chrono::Utc;
    use mongodb::bson::doc;
    use mongodb::Client as mdbClient;
    use rocket::http::{ContentType, Header, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::json;
    // use serde_json::json;

    use crate::auth::generate_jwt;
    use crate::rocket;

    lazy_static! {
        static ref MAIN_USER_TOKEN: String = {
            let main_user_id = "63d3df1e99677d2661245b5c";
            match generate_jwt(main_user_id) {
                Ok(token) => token,
                Err(e) => panic!("{e}"),
            }
        };
        static ref SECOND_USER_TOKEN: String = {
            let second_user_id = "63cf52e3c0ed8d6217cbb98f";
            match generate_jwt(second_user_id) {
                Ok(token) => token,
                Err(e) => panic!("{e}"),
            }
        };
    }

    async fn cleanup(title: &str) -> Result<(), mongodb::error::Error> {
        let mongo_db = match mdbClient::with_uri_str("mongodb://localhost:27017/e-form").await {
            Ok(client) => client.database("e-form"),
            Err(e) => panic!("{e}"),
        };

        let _result: mongodb::results::DeleteResult = mongo_db
            .collection::<Document>("forms")
            .delete_many(doc! {"title": title}, None)
            .await?;

        Ok(())
    }

    #[async_test]
    async fn get_form_as_main_user() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .get(uri!(crate::routes::form::get_form(
                "63d1eb130d6b861224602a68"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *MAIN_USER_TOKEN),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        println!("{:?}", response.body());
        // assert!(cleanup("test@example.com").await.is_ok());
    }

    #[async_test]
    async fn get_form_as_random() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .get(uri!(crate::routes::form::get_form(
                "63d1eb130d6b861224602a68"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *SECOND_USER_TOKEN),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
        println!("{:?}", response.body());
        // assert!(cleanup("test@example.com").await.is_ok());
    }

    #[async_test]
    async fn get_form_as_anon() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client
            .get(uri!(crate::routes::form::get_form(
                "63d1eb130d6b861224602a68"
            )))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
        println!("{:?}", response.body());
        // assert!(cleanup("test@example.com").await.is_ok());
    }

    #[async_test]
    async fn post_form() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .post(uri!(crate::routes::form::post_form))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *MAIN_USER_TOKEN),
            ))
            .body(
                json!({
                    "title": "TEST FORM",
                    "description": "TEST FORM FOR DEVELOPMENT DO NOT TOUCH",
                    "state": "Private",
                    "questions": [
                      {
                        "number": 1,
                        "text": "Hello",
                        "kind": "TextAnswer",
                        "options": null
                      },
                      {
                        "number": 2,
                        "text": "Hello, world.",
                        "kind": "MultipleChoice",
                        "options": [
                          "Choice 1",
                          "Choice 2"
                        ]
                      }
                    ],
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        println!("{:?}", response.body());

        assert!(cleanup("TEST FORM").await.is_ok());
    }

    #[async_test]
    async fn put_form() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .put(uri!(crate::routes::form::put_form(
                "63d61a5319a9f178d5652b4a"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *MAIN_USER_TOKEN),
            ))
            .body(
                json!({
                    "title": "TEST PUT FORM",
                    "description": "PUT FORM TESTING",
                    "state": "Public",
                    "questions": [
                      {
                        "number": 1,
                        "text": "Hello",
                        "kind": "TextAnswer",
                        "options": null
                      },
                      {
                        "number": 2,
                        "text": "Hello, world.",
                        "kind": "MultipleChoice",
                        "options": [
                          "Choice 1",
                          "Choice 2"
                        ]
                      }
                    ],
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        println!("{:?}", response.body());

        let response = client
            .put(uri!(crate::routes::form::put_form(
                "63d61a5319a9f178d5652b4a"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *MAIN_USER_TOKEN),
            ))
            .body(
                json!({
                    "title": "TEST PUT FORM",
                    "description": "TEST FORM FOR DEVELOPMENT DO NOT TOUCH",
                    "state": "Anonymous",
                    "questions": [
                      {
                        "number": 1,
                        "text": "Hello",
                        "kind": "TextAnswer",
                        "options": null
                      },
                      {
                        "number": 2,
                        "text": "Hello, world.",
                        "kind": "MultipleChoice",
                        "options": [
                          "Choice 1",
                          "Choice 2"
                        ]
                      }
                    ],
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        println!("{:?}", response.body());

        // assert!(cleanup("TEST FORM").await.is_ok());
    }

    #[async_test]
    async fn put_form_as_non_owner() {
        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .put(uri!(crate::routes::form::put_form(
                "63d61a5319a9f178d5652b4a"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *SECOND_USER_TOKEN),
            ))
            .body(
                json!({
                    "title": "TEST PUT FORM",
                    "description": "PUT FORM TESTING",
                    "state": "Public",
                    "questions": [
                      {
                        "number": 1,
                        "text": "Hello",
                        "kind": "TextAnswer",
                        "options": null
                      },
                      {
                        "number": 2,
                        "text": "Hello, world.",
                        "kind": "MultipleChoice",
                        "options": [
                          "Choice 1",
                          "Choice 2"
                        ]
                      }
                    ],
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
        println!("{:?}", response.body());

        let response = client
            .put(uri!(crate::routes::form::put_form(
                "63d61a5319a9f178d5652b4a"
            )))
            .header(ContentType::JSON)
            .body(
                json!({
                    "title": "TEST PUT FORM",
                    "description": "TEST FORM FOR DEVELOPMENT DO NOT TOUCH",
                    "state": "Anonymous",
                    "questions": [
                      {
                        "number": 1,
                        "text": "Hello",
                        "kind": "TextAnswer",
                        "options": null
                      },
                      {
                        "number": 2,
                        "text": "Hello, world.",
                        "kind": "MultipleChoice",
                        "options": [
                          "Choice 1",
                          "Choice 2"
                        ]
                      }
                    ],
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
        println!("{:?}", response.body());

        // assert!(cleanup("TEST FORM").await.is_ok());
    }

    #[async_test]
    async fn delete_form() {
        let mongo_db = match mdbClient::with_uri_str("mongodb://localhost:27017/e-form").await {
            Ok(client) => client.database("e-form"),
            Err(e) => panic!("{e}"),
        };

        let _res = mongo_db
            .collection("forms")
            .insert_one(
                doc! {
                  "_id": ObjectId::from_str("63d61a5319a9f178d5652b4b").unwrap(),
                  "owner": ObjectId::from_str("63d3df1e99677d2661245b5c").unwrap(),
                  "title": "TEST PUT FORM",
                  "description": "TEST FORM FOR DEVELOPMENT DO NOT TOUCH",
                  "state": "Anonymous",
                  "questions": [
                    {
                      "number": 1,
                      "text": "Hello",
                      "kind": "TextAnswer",
                      "options": null
                    },
                    {
                      "number": 2,
                      "text": "Hello, world.",
                      "kind": "MultipleChoice",
                      "options": [
                        "Choice 1",
                        "Choice 2"
                      ]
                    }
                  ],
                  "created_at": Utc::now()
                },
                None,
            )
            .await;

        println!("{_res:?}");

        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .delete(uri!(crate::routes::form::delete_form(
                "63d61a5319a9f178d5652b4b"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *MAIN_USER_TOKEN),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        println!("{:?}", response.body());
    }

    #[async_test]
    async fn delete_form_as_non_owner() {
        let mongo_db = match mdbClient::with_uri_str("mongodb://localhost:27017/e-form").await {
            Ok(client) => client.database("e-form"),
            Err(e) => panic!("{e}"),
        };

        let _res = mongo_db
            .collection("forms")
            .insert_one(
                doc! {
                "_id": ObjectId::from_str("63d61a5319a9f178d5652b4b").unwrap(),
                  "owner": ObjectId::from_str("63d3df1e99677d2661245b5c").unwrap(),
                  "title": "TEST PUT FORM",
                  "description": "TEST FORM FOR DEVELOPMENT DO NOT TOUCH",
                  "state": "Anonymous",
                  "questions": [
                    {
                      "number": 1,
                      "text": "Hello",
                      "kind": "TextAnswer",
                      "options": null
                    },
                    {
                      "number": 2,
                      "text": "Hello, world.",
                      "kind": "MultipleChoice",
                      "options": [
                        "Choice 1",
                        "Choice 2"
                      ]
                    }
                  ],
                  "created_at": Utc::now()
                },
                None,
            )
            .await;

        let client = Client::tracked(rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .delete(uri!(crate::routes::form::delete_form(
                "63d61a5319a9f178d5652b4b"
            )))
            .header(ContentType::JSON)
            .header(Header::new(
                "Authorization",
                format!("Bearer {}", *SECOND_USER_TOKEN),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
        println!("{:?}", response.body());

        let response = client
            .delete(uri!(crate::routes::form::delete_form(
                "63d61a5319a9f178d5652b4a"
            )))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
        println!("{:?}", response.body());
    }
}
