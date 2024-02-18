use crate::{
    jwt_auth,
    payment::model::PaymentModel,
    payment::schema::{
        LoginPaymentSchema,
        TokenClaims,
        CreatePaymentSchema,
        FilterOptions,
        UpdatePaymentSchema,
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
use chrono::prelude::*;
use serde_json::json;
use sqlx::Row;

#[get("/")]
pub async fn payment_list_handler(
    opts: web::Query<FilterOptions>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
            PaymentModel,
            "SELECT * FROM payments ORDER by id LIMIT $1 OFFSET $2",
            limit as i32,
            offset as i32
        )
        .fetch_all(&data.db).await;

    if query_result.is_err() {
        let message = "Something bad happened while fetching all payment items";
        return HttpResponse::InternalServerError().json(
            json!({"status": "error","message": message})
        );
    }

    let payments = query_result.unwrap();

    let json_response =
        serde_json::json!({
        "status": "success",
        "results": payments.len(),
        "payments": payments
    });
    HttpResponse::Ok().json(json_response)
}

#[post("/")]
async fn create_payment_handler(
    body: web::Json<CreatePaymentSchema>,
    data: web::Data<AppState>
) -> impl Responder {
    let query_result = sqlx
        ::query_as!(
            PaymentModel,
            "INSERT INTO payments (name,description,price,user_id, category_id) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            body.name,
            body.description,
            body.price,
            body.userId,
            body.categoryId,
        )
        .fetch_one(&data.db).await;

    match query_result {
        Ok(payment) => {
            let payment_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "payment": payment
            })});

            return HttpResponse::Ok().json(payment_response);
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(
                serde_json::json!({"status": "error","message": format!("{:?}", e)})
            );
        }
    }
}

#[get("/{id}")]
async fn get_payment_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let payment_id = path.into_inner();
    let query_result = sqlx
        ::query_as!(PaymentModel, "SELECT * FROM payments WHERE id = $1", payment_id)
        .fetch_one(&data.db).await;

    match query_result {
        Ok(payment) => {
            let payment_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "payment": payment
            })});

            return HttpResponse::Ok().json(payment_response);
        }
        Err(_) => {
            let message = format!("Payment with ID: {} not found", payment_id);
            return HttpResponse::NotFound().json(
                serde_json::json!({"status": "fail","message": message})
            );
        }
    }
}

#[patch("/{id}")]
async fn edit_payment_handler(
    path: web::Path<uuid::Uuid>,
    body: web::Json<UpdatePaymentSchema>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let payment_id = path.into_inner();
    let query_result = sqlx
        ::query_as!(PaymentModel, "SELECT * FROM payments WHERE id = $1", payment_id)
        .fetch_one(&data.db).await;

    if query_result.is_err() {
        let message = format!("Payment with ID: {} not found", payment_id);
        return HttpResponse::NotFound().json(
            serde_json::json!({"status": "fail","message": message})
        );
    }

    let now = Utc::now();
    let query_result = sqlx
        ::query_as!(
            PaymentModel,
            "UPDATE payments SET name = $1, description = $2, price = $3, user_id = $4, category_id = $5 updated_at = 6$ WHERE id = $7 RETURNING *",
            body.name,
            body.description,
            body.price,
            body.userId,
            body.categoryId,
            now,
            payment_id
        )
        .fetch_one(&data.db).await;

    match query_result {
        Ok(payment) => {
            let payment_response =
                serde_json::json!({"status": "success","data": serde_json::json!({
                "payment": payment
            })});

            return HttpResponse::Ok().json(payment_response);
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
async fn delete_payment_handler(
    path: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    _: jwt_auth::JwtMiddleware
) -> impl Responder {
    let payment_id = path.into_inner();
    let rows_affected = sqlx
        ::query!("DELETE FROM payments WHERE id = $1", payment_id)
        .execute(&data.db).await
        .unwrap()
        .rows_affected();

    if rows_affected == 0 {
        let message = format!("Payment with ID: {} not found", payment_id);
        return HttpResponse::NotFound().json(json!({"status": "fail","message": message}));
    }

    HttpResponse::NoContent().finish()
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web
        ::scope("/payments")
        .service(payment_list_handler)
        .service(create_payment_handler)
        .service(get_payment_handler)
        .service(edit_payment_handler)
        .service(delete_payment_handler)

    conf.service(scope);
}
