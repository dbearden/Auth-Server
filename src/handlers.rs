use actix_web::{post, web, HttpResponse, Responder};
use argonautica::{Hasher, Verifier};
use futures::compat;
use log::{error, info, warn};
use uuid::Uuid;

use crate::models::Credentials;
use crate::DbPool;
use crate::{REGEX, SETTINGS};

async fn hash(password: &str) -> Result<String, argonautica::Error> {
    let h = Hasher::default()
        .with_password(password)
        .with_secret_key(SETTINGS.get_str("secret").unwrap())
        .hash_non_blocking();
    compat::Compat01As03::new(h).await
}
async fn verify(hash: &str, password: &str) -> Result<bool, argonautica::Error> {
    let v = Verifier::default()
        .with_hash(hash)
        .with_password(password)
        .with_secret_key(SETTINGS.get_str("secret").unwrap())
        .verify_non_blocking();
    compat::Compat01As03::new(v).await
}

#[post("api/login")]
async fn login(form: web::Form<Credentials>, pool: web::Data<DbPool>) -> impl Responder {
    match pool.get().await {
        Ok(client) => {
            let query = match client
                .prepare("SELECT user_id, password_hash FROM users WHERE username = $1;")
                .await
            {
                Ok(statement) => statement,
                Err(error) => {
                    error!("Error preparing statment: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            let row = match client.query_one(&query, &[&form.username]).await {
                Ok(row) => row,
                Err(error) => {
                    error!("Query error: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            let hash: &str = row.get("password_hash");
            let id: i32 = row.get("user_id");
            let verified = match verify(hash, &form.password).await {
                Ok(verified) => verified,
                Err(error) => {
                    error!("Hash verification error: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            if verified {
                let token = Uuid::new_v4().to_simple();
                let insert = client
                    .prepare("INSERT INTO logins VALUES($1, 'now', $2)")
                    .await
                    .unwrap();
                client
                    .execute(&insert, &[&id, &token.to_string()])
                    .await
                    .unwrap();
                HttpResponse::Ok().body(format!("Logged In {}: {}", &form.username, token))
            } else {
                HttpResponse::BadRequest().body("Invalid login information")
            }
        }
        Err(error) => {
            error!("Pool error: {}", error);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("api/register")]
async fn register(form: web::Form<Credentials>, pool: web::Data<DbPool>) -> impl Responder {
    if !REGEX.is_match(&form.password) {
        return HttpResponse::BadRequest().body("Invalid Password");
    }
    match pool.get().await {
        Ok(client) => {
            let stmt = match client
                .prepare("INSERT INTO users (username, password_hash) VALUES($1, $2);")
                .await
            {
                Ok(statement) => statement,
                Err(error) => {
                    error!("Error preparing statement: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            let hash = match hash(&form.password).await {
                Ok(hash) => hash,
                Err(error) => {
                    error!("Hashing error: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            match client.execute(&stmt, &[&form.username, &hash]).await {
                Ok(_) => (),
                Err(error) => {
                    error!("Query error: {}", error);
                    return HttpResponse::InternalServerError().finish();
                }
            };
            HttpResponse::Created().body(format!("Registered {}", form.username))
        }
        Err(error) => {
            error!("Pool error: {}", error);
            HttpResponse::InternalServerError().finish()
        }
    }
}
