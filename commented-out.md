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
