use tonic::{transport::Server, Request, Response, Status};
use category::category_server::{Category, CategoryServer};
use category::{CategoryModel, GetCategoryRequest, GetCategoryResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use actix_web::web;

pub mod category {
    tonic::include_proto!("category");
}

#[derive(Debug)]
pub struct CategoryService {
    db_pool: PgPool,
}

impl CategoryService {
    pub fn new(db_pool: PgPool) -> Self {
        CategoryService { db_pool }
    }
}

#[tonic::async_trait]
impl Category for CategoryService {
    async fn get_category(
        &self,
        request: Request<GetCategoryRequest>,
    ) -> Result<Response<GetCategoryResponse>, Status> {
        println!("Received request from: {:?}", request);

        let category_id: Uuid = Uuid::parse_str(&request.get_ref().id)
            .map_err(|_| Status::invalid_argument("Invalid UUID format"))?;

        let query_result = sqlx::query_as!(
            CategoryModel,
            "SELECT * FROM categories WHERE id = $1",
            category_id
        )
        .fetch_one(&self.db_pool)
        .await;

        match query_result {
            Ok(category) => {
                let response = GetCategoryResponse {
                    category: Some(category),
                    message: "Category found successfully".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(_) => {
                let message = "Category not found".to_string();
                Err(Status::not_found(message))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = PgPool::connect(&database_url).await?;

    let addr = "[::1]:50051".parse()?;
    let category_service = CategoryService::new(db_pool.clone());

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(CategoryServer::new(category_service))
        .serve(addr)
        .await?;

    Ok(())
}
