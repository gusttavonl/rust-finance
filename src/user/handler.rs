use crate::{
    jwt_auth,
    user::model::UserModel,
    user::schema::{
        LoginUserSchema,
        TokenClaims,
        CreateUserSchema,
        FilterOptions,
        UpdateUserSchema,
    },
    AppState,
};
use actix_web::{
    cookie::{ time::Duration as ActixWebDuration, Cookie },
    delete,
    get,
    patch,
    post,
    web,
    HttpResponse,
    Responder,
};
use argon2::{
    password_hash::{ rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString },
    Argon2,
};
use jsonwebtoken::{ encode, EncodingKey, Header };
use chrono::prelude::*;
use serde_json::json;
use sqlx::Row;

#[get("/")]
pub async fn user_list_handler(
    opts: web::Query<FilterOptions>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
            UserModel,
            "SELECT * FROM users ORDER by id LIMIT $1 OFFSET $2",
            limit as i32,
            offset as i32
        )
        .fetch_all(&data.db).await;

    if query_result.is_err() {
        let message = "Something bad happened while fetching all user items";
        return HttpResponse::InternalServerError().json(
            json!({"status": "error","message": message})
        );
    }

    let users = query_result.unwrap();

    let json_response =
        serde_json::json!({
        "status": "success",
        "results": users.len(),
        "users": users
    });
    HttpResponse::Ok().json(json_response)
}

#[post("/")]
async fn create_user_handler(
    body: web::Json<CreateUserSchema>,
    data: web::Data<AppState>
) -> impl Responder {
    let exists: bool = sqlx
        ::query("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(body.email.to_owned())
        .fetch_one(&data.db).await
        .unwrap()
        .get(0);

    if exists {
        return HttpResponse::Conflict().json(
            serde_json::json!({"status": "fail","message": "User with that email already exists"})
        );
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .expect("Error while hashing password")
        .to_string();
    let query_result = sqlx
        ::query_as!(
            UserModel,
            "INSERT INTO users (name,email,password) VALUES ($1, $2, $3) RETURNING *",
            body.name,
            body.email.to_string(),
            hashed_password
        )
        .fetch_one(&data.db).await;

    match query_result {
        Ok(user) => {
            let user_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "user": user
            })});

            return HttpResponse::Ok().json(user_response);
        }
        Err(e) => {
            if e.to_string().contains("duplicate key value violates unique constraint") {
                return HttpResponse::BadRequest().json(
                    serde_json::json!({"status": "fail","message": "User with that title already exists"})
                );
            }

            return HttpResponse::InternalServerError().json(
                serde_json::json!({"status": "error","message": format!("{:?}", e)})
            );
        }
    }
}

#[get("/{id}")]
async fn get_user_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let user_id = path.into_inner();
    let query_result = sqlx
        ::query_as!(UserModel, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(&data.db).await;

    match query_result {
        Ok(user) => {
            let user_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "user": user
            })});

            return HttpResponse::Ok().json(user_response);
        }
        Err(_) => {
            let message = format!("User with ID: {} not found", user_id);
            return HttpResponse::NotFound().json(
                serde_json::json!({"status": "fail","message": message})
            );
        }
    }
}

#[patch("/{id}")]
async fn edit_user_handler(
    path: web::Path<uuid::Uuid>,
    body: web::Json<UpdateUserSchema>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let user_id = path.into_inner();
    let query_result = sqlx
        ::query_as!(UserModel, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(&data.db).await;

    if query_result.is_err() {
        let message = format!("User with ID: {} not found", user_id);
        return HttpResponse::NotFound().json(
            serde_json::json!({"status": "fail","message": message})
        );
    }

    let now = Utc::now();
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .expect("Error while hashing password")
        .to_string();
    let query_result = sqlx
        ::query_as!(
            UserModel,
            "UPDATE users SET name = $1, email = $2, password = $3 updated_at = 4$ WHERE id = $5 RETURNING *",
            body.name.to_string(),
            body.email,
            hashed_password,
            now,
            user_id
        )
        .fetch_one(&data.db).await;

    match query_result {
        Ok(user) => {
            let user_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "user": user
            })});

            return HttpResponse::Ok().json(user_response);
        }
        Err(err) => {
            let message = format!("Error: {:?}", err);
            return HttpResponse::InternalServerError().json(
                serde_json::json!({"status": "error","message": message})
            );
        }
    }
}

#[delete("/{id}")]
async fn delete_user_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let user_id = path.into_inner();
    let rows_affected = sqlx
        ::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&data.db).await
        .unwrap()
        .rows_affected();

    if rows_affected == 0 {
        let message = format!("User with ID: {} not found", user_id);
        return HttpResponse::NotFound().json(json!({"status": "fail","message": message}));
    }

    HttpResponse::NoContent().finish()
}

#[post("/login")]
async fn login_user_handler(
    body: web::Json<LoginUserSchema>,
    data: web::Data<AppState>
) -> impl Responder {
    let query_result = sqlx
        ::query_as!(UserModel, "SELECT * FROM users WHERE email = $1", body.email)
        .fetch_one(&data.db).await;

    let user = match query_result {
        Ok(ref user) => {
            let parsed_hash = PasswordHash::new(&user.password).unwrap();
            Argon2::default()
                .verify_password(body.password.as_bytes(), &parsed_hash)
                .map_or(false, |_| true)
        }
        Err(_) => false,
    };

    if !user {
        return HttpResponse::BadRequest().json(
            json!({"status": "fail", "message": "Invalid email or password"})
        );
    }

    let user = query_result.unwrap();

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user.id.to_string(),
        exp,
        iat,
    };

    let secret_key = "my_ultra_secure_secret";
    let secret_key_bytes = secret_key.as_bytes();

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key_bytes)
    ).unwrap();

    let cookie = Cookie::build("token", token.to_owned())
        .path("/")
        .max_age(ActixWebDuration::new(60 * 60, 0))
        .http_only(true)
        .finish();

    let role = sqlx::query!(
        "SELECT * FROM roles WHERE id = $1",
        user.role_id
    )
    .fetch_one(&data.db)
    .await
    .map(|row| {
        json!({
            "id": row.id,
            "name": row.name,
            "description": row.description,
            "admin": row.admin,
            "super_admin": row.super_admin
        })
    })
    .unwrap_or_else(|_| {
        json!({
            "id": user.role_id,
            "name": "Unknown Role",
            "description": "Role details not found",
            "admin": false,
            "super_admin": false
        })
    });

    let mut user_json = serde_json::to_value(user).unwrap();
    user_json["role"] = role;

    HttpResponse::Ok()
        .cookie(cookie)
        .json(json!({"status": "success", "token": token, "user": user_json}))
}

#[get("/logout")]
async fn logout_handler(_: jwt_auth::JwtMiddleware) -> impl Responder {
    let cookie = Cookie::build("token", "")
        .path("/")
        .max_age(ActixWebDuration::new(-1, 0))
        .http_only(true)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(json!({"status": "success"}))
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web
        ::scope("/users")
        .service(user_list_handler)
        .service(create_user_handler)
        .service(get_user_handler)
        .service(edit_user_handler)
        .service(delete_user_handler)
        .service(login_user_handler)
        .service(logout_handler);

    conf.service(scope);
}
