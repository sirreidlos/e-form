# SQLX user query example

```rs
match sqlx::query("SELECT * FROM `users` WHERE `email` = ?")
    .bind(&data.email)
    .fetch_one(db.inner())
    .await
{
    Ok(_) => {
        return Custom(
            Status::Conflict,
            json!( {
                "message": "An existing account has been made using this email.",
            }),
        )
    }
    Err(e) => {
        if !matches!(e, sqlx::Error::RowNotFound) {
            println!("{e}");
            return Custom(
                Status::InternalServerError,
                json!( {
                    "message": "An error has occurred.",
                }),
            );
        }
    }
}
```

# Edit Form (useless)

```rs
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
```

# SQLX CONNECT

```rs
let db_url = "mysql://root@localhost/e_form";
let pool = match MySqlPool::connect(db_url).await {
    Ok(pool) => pool,
    Err(error) => panic!("Failed to connect to database: {error}"),
};
```

# Testing with JWT

```rs
use auth::generate_jwt;
#[get("/<user_id>")]
pub fn test_token(user_id: &str) -> Option<Value> {
    // Some(json!({ "token": generate_jwt(user_id) }))
    match generate_jwt(user_id) {
        Ok(token) => Some(json!({ "token": token })),
        Err(_) => None,
    }
}

use auth::Auth;
#[get("/attempt")]
fn attempt(user_id: Auth) -> Option<Value> {
    Some(json!({ "token": user_id.0 }))
}
```

```rs
db.collection("forms")
    .find_one(doc! {"_id": id}, None)
    .await
    .map(|form| {
        form.ok_or(mongodb::error::Error::from(
            mongodb::error::ErrorKind::DocumentNotFound,
        ))
    })
    .map_err(|_| {
        Custom(
            Status::InternalServerError,
            json!({
                "message": "An internal server error has occured."
            }),
        )
    })
```
