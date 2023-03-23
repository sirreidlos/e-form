use std::collections::HashMap;

use bson::{
    serde_helpers::{deserialize_hex_string_from_object_id, serialize_hex_string_as_object_id},
    Document,
};

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use mongodb::Database;
use mongodb::{bson::oid::ObjectId, Cursor};
use rocket::tokio::select;
use rocket::{
    http::Status,
    response::status::Custom,
    serde::json::Json,
    tokio::sync::broadcast::{error::RecvError, Sender},
    State,
};
use rocket::{
    response::stream::{Event, EventStream},
    Shutdown,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::Auth;
use crate::routes::form::{Form, QuestionType};
use crate::routes::{find_form_by_id, object_id_from_string};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    _id: String,
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    responder: String,
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id"
    )]
    form: String,
    answers: Vec<Answer>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Answer {
    number: u32,
    input: Option<String>,
    selected_options: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseData {
    pub answers: Vec<Answer>,
}

pub async fn find_response_by_id(
    id: &str,
    db: &State<Database>,
) -> Result<Response, Custom<Value>> {
    let obj_id = object_id_from_string(id)?;
    match db
        .collection("responses")
        .find_one(doc! {"_id": obj_id}, None)
        .await
    {
        Ok(Some(form)) => Ok(form),
        Ok(None) => Err(Custom(
            Status::NotFound,
            json!({
                "message": "Response not found."
            }),
        )),
        Err(e) => {
            println!("{e} in find_response_by_id");
            Err(Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            ))
        }
    }
}

async fn find_multiple_responses_by_id(
    id: ObjectId,
    db: &State<Database>,
) -> mongodb::error::Result<Vec<Response>> {
    let mut cursor: Cursor<Response> = db
        .collection("responses")
        .find(doc! { "form": id }, None)
        .await?;

    let mut responses = vec![];

    while cursor.advance().await? {
        let current = cursor.deserialize_current()?;
        responses.push(current)
    }

    Ok(responses)
}

fn serialize_responses_to_json(responses: Vec<Response>) -> Vec<Value> {
    let mut json_responses = vec![];

    for res in responses {
        json_responses.push(json!({
            "_id": res._id,
            "responder": res.responder,
            "form": res.form,
            "answers": res.answers,
            "created_at": res.created_at.to_rfc3339(),
        }))
    }

    json_responses
}

#[get("/stream/<id>")]
pub async fn response_stream(
    id: String,
    user_id: Auth,
    db: &State<Database>,
    queue: &State<Sender<Response>>,
    mut end: Shutdown,
) -> Result<EventStream![], Custom<Value>> {
    let form: Form = find_form_by_id(&id, db).await?;

    if form.owner != user_id.0 {
        return Err(Custom(
            Status::Forbidden,
            json!({
                "message": "You are not the owner of this form."
            }),
        ));
    }

    let mut rx = queue.subscribe();

    Ok(EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => {
                        if msg.form != id {
                            continue;
                        }

                        json!({
                            "_id": msg._id,
                            "responder": msg.responder,
                            "form": msg.form,
                            "answers": msg.answers,
                            "created_at": msg.created_at.to_rfc3339(),
                    })
                    },
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    })
}

#[get("/stream/<_>", rank = 2)]
pub async fn response_stream_as_anon() -> Custom<Value> {
    Custom(
        Status::Unauthorized,
        json!({"message": "You are not logged in."}),
    )
}

#[get("/response/<id>")]
pub async fn get_response(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
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

    let obj_id = object_id_from_string(&form._id).unwrap();

    let responses = match find_multiple_responses_by_id(obj_id, db).await {
        Ok(res) => res,
        Err(e) => {
            println!("{e} in get response responses");
            return Custom(
                Status::NoContent,
                json!({"message": "An error has occurred."}),
            );
        }
    };

    Custom(Status::Ok, json!(serialize_responses_to_json(responses)))
}

#[post("/response/<id>", format = "json", data = "<data>")]
pub async fn post_response(
    id: String,
    user_id: Auth,
    data: Json<ResponseData>,
    db: &State<Database>,
    queue: &State<Sender<Response>>,
) -> Custom<Value> {
    let form: Form = match find_form_by_id(&id, db).await {
        Ok(form) => form,
        Err(e) => {
            return e;
        }
    };

    for (i, question) in form.questions.iter().enumerate() {
        if i >= data.answers.len() {
            return Custom(
                Status::UnprocessableEntity,
                json!({"message": "Form has more questions than answers provided."}),
            );
        }

        let answer = &data.answers[i];

        if answer.number != question.number {
            return Custom(
                Status::UnprocessableEntity,
                json!({
                    "message":
                        format!(
                            "Answer number {} does not match question number {}",
                            answer.number, question.number
                        )
                }),
            );
        }

        if question.kind == QuestionType::Checkboxes {
            if let Some(options) = &question.options {
                if answer.selected_options.is_none() {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message": format!("Answer number {} is missing input", answer.number)
                        }),
                    );
                }

                let input: &Vec<String> = answer.selected_options.as_ref().unwrap();
                for i in input {
                    if !options.contains(&i.to_owned()) {
                        return Custom(
                            Status::UnprocessableEntity,
                            json!({
                                "message":
                                    format!(
                                        "Answer number {} input '{}' is not in options {:?}",
                                        answer.number, i, options
                                    )
                            }),
                        );
                    }
                }
                // let input: Vec<String> = input.iter().map(|s| s.to_owned()).collect();
            }
            continue;
        }

        if question.kind == QuestionType::Date {
            let input = answer.input.as_ref().unwrap();
            let split: Vec<&str> = input.split('/').collect();

            if split.len() != 3 {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
                                answer.number, input
                            )
                    }),
                );
            }

            match split[0].parse::<u32>() {
                Ok(_) => (),
                Err(_) => {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
                                answer.number, input
                            )
                        }),
                    )
                }
            }

            match split[1].parse::<u32>() {
                Ok(m) => {
                    let r = 1..=12;
                    if !std::ops::RangeInclusive::contains(&r, &m) {
                        return Custom(
                            Status::UnprocessableEntity,
                            json!({
                                "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (mm accepts 01-12).",
                                answer.number, input
                            )
                            }),
                        );
                    }
                }
                Err(_) => {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
                                answer.number, input
                            )
                        }),
                    );
                }
            }

            match split[2].parse::<u32>() {
                Ok(d) => {
                    let r = 1..=31;
                    if !std::ops::RangeInclusive::contains(&r, &d) {
                        return Custom(
                            Status::UnprocessableEntity,
                            json!({
                                "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (dd accepts 01-31).",
                                answer.number, input
                            )
                            }),
                        );
                    }
                }
                Err(_) => {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
                                answer.number, input
                            )
                        }),
                    );
                }
            }

            // if let Ok(y) = split[0].parse::<u32>() {
            //     y
            // } else {
            //     return Custom(
            //         Status::UnprocessableEntity,
            //         json!({
            //             "message":
            //                 format!(
            //                     "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
            //                     answer.number, input
            //                 )
            //         }),
            //     );
            // };
            // if let Ok(m) = split[1].parse::<u32>() {
            //     if m > 12 {
            //         return Custom(
            //             Status::UnprocessableEntity,
            //             json!({
            //                 "message":
            //                 format!(
            //                     "Answer number {} input '{}' has invalid input (mm accepts 01-12).",
            //                     answer.number, input
            //                 )
            //             }),
            //         );
            //     }
            // } else {
            //     return Custom(
            //         Status::UnprocessableEntity,
            //         json!({
            //             "message":
            //                 format!(
            //                     "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
            //                     answer.number, input
            //                 )
            //         }),
            //     );
            // };
            // if let Ok(d) = split[2].parse::<u32>() {
            //     if d > 31 {
            //         return Custom(
            //             Status::UnprocessableEntity,
            //             json!({
            //                 "message":
            //                 format!(
            //                     "Answer number {} input '{}' has invalid input (mm accepts 01-12).",
            //                     answer.number, input
            //                 )
            //             }),
            //         );
            //     }
            // } else {
            //     return Custom(
            //         Status::UnprocessableEntity,
            //         json!({
            //             "message":
            //                 format!(
            //                     "Answer number {} input '{}' has invalid input (expected yyyy/mm/dd).",
            //                     answer.number, input
            //                 )
            //         }),
            //     );
            // };

            continue;
        }

        if question.kind == QuestionType::Time {
            let input = answer.input.as_ref().unwrap();
            let split: Vec<&str> = input.split(':').collect();

            if split.len() != 2 {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected hh:mm).",
                                answer.number, input
                            )
                    }),
                );
            }

            if let Ok(h) = split[0].parse::<u32>() {
                if h > 23 {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (hh accepts 00-23).",
                                answer.number, input
                            )
                        }),
                    );
                }
            } else {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected hh:mm).",
                                answer.number, input
                            )
                    }),
                );
            };

            if let Ok(m) = split[1].parse::<u32>() {
                if m > 59 {
                    return Custom(
                        Status::UnprocessableEntity,
                        json!({
                            "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (mm accepts 00-59).",
                                answer.number, input
                            )
                        }),
                    );
                }
            } else {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message":
                            format!(
                                "Answer number {} input '{}' has invalid input (expected hh:mm).",
                                answer.number, input
                            )
                    }),
                );
            };

            continue;
        }

        if let Some(options) = &question.options {
            if answer.input.is_none() {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message": format!("Answer number {} is missing input", answer.number)
                    }),
                );
            }

            let input = answer.input.as_ref().unwrap();
            if !options.contains(input) {
                return Custom(
                    Status::UnprocessableEntity,
                    json!({
                        "message":
                            format!(
                                "Answer number {} input '{}' is not in options {:?}",
                                answer.number, input, options
                            )
                    }),
                );
            }
        }
    }

    let obj_id = ObjectId::new();

    let res_struct = Response {
        _id: obj_id.to_string(),
        responder: user_id.0,
        form: id,
        answers: data.answers.clone(),
        created_at: Utc::now(),
    };

    match db
        .collection("responses")
        .insert_one(res_struct, None)
        .await
    {
        Ok(_) => (),
        Err(e) => {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!({"message": "An internal server error has occurred."}),
            );
        }
    };

    match db
        .collection("responses")
        .find_one(doc! {"_id": obj_id}, None)
        .await
    {
        Ok(response) => {
            if response.is_none() {
                return Custom(
                    Status::InternalServerError,
                    json!({"message": "This error should and will never be called."}),
                );
            }
            let _res = queue.send(response.unwrap());
            Custom(Status::Created, json!({"message": "Response sent."}))
        }
        Err(e) => {
            println!("{e}");
            Custom(
                Status::InternalServerError,
                json!({"message": "This error should and will never be called."}),
            )
        }
    }
}

#[delete("/response/<id>")]
pub async fn delete_response(id: String, user_id: Auth, db: &State<Database>) -> Custom<Value> {
    let response: Response = match find_response_by_id(&id, db).await {
        Ok(response) => response,
        Err(e) => {
            return e;
        }
    };

    let form: Form = match find_form_by_id(&response.form, db).await {
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
        .delete_one(doc! {"_id": response._id}, None)
        .await
    {
        Ok(_) => Custom(Status::Ok, json!({"message": "Response deleted."})),
        Err(e) => {
            println!("{e} in delete_response");
            Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            )
        }
    }
}

#[get("/chart/<id>")]
pub async fn response_chart(
    id: String,
    user_id: Auth,
    db: &State<Database>,
    queue: &State<Sender<Response>>,
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

    let obj_id = object_id_from_string(&id).unwrap();
    let responses = match find_multiple_responses_by_id(obj_id, db).await {
        Ok(responses) => {
            let mut responses_summary: HashMap<usize, HashMap<String, usize>> = HashMap::new();
            for response in responses {
                for answer in response.answers {
                    if let Some(input) = answer.input {
                        let index = (answer.number - 1) as usize;

                        responses_summary
                            .entry(index + 1)
                            .and_modify(|map| {
                                map.entry(input.clone())
                                    .and_modify(|v| *v += 1)
                                    .or_insert(1);
                            })
                            .or_insert(HashMap::from([(input, 1)]));

                        continue;
                    }

                    if let Some(inputs) = answer.selected_options {
                        let index = (answer.number - 1) as usize;

                        for input in inputs {
                            responses_summary
                                .entry(index + 1)
                                .and_modify(|map| {
                                    map.entry(input.clone())
                                        .and_modify(|v| *v += 1)
                                        .or_insert(1);
                                })
                                .or_insert(HashMap::from([(input, 1)]));
                        }

                        continue;
                    }
                }
            }

            responses_summary
        }
        Err(e) => {
            eprintln!(
                "{:?} - {e:?}",
                std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)
            );
            return Custom(
                Status::InternalServerError,
                json!({
                    "message": "An internal server error has occurred."
                }),
            );
        }
    };

    Custom(Status::Ok, json!(responses))
}

#[get("/chart/<_>", rank = 2)]
pub async fn response_chart_as_anon() -> Custom<Value> {
    Custom(
        Status::Unauthorized,
        json!({"message": "You are not logged in."}),
    )
}
