mod user;
mod category;
mod payment;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{http::header, web, App, HttpServer};
use dotenv::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub struct AppState {
    db: Pool<Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("Failed to connect to the database: {:?}. url: {:?}", err, database_url);
            std::process::exit(1);
        }
    };

    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => println!("Migrations executed successfully."),
        Err(e) => eprintln!("Error executing migrations: {}", e),
    };

    let pg_pool_move = pool.clone();
    tokio::spawn(async move {
        let result = amqp::lib::run(pg_pool_move).await;
        if result.is_err() {
            println!("{}", result.unwrap_err().to_string());
            std::process::exit(1)
        }
    });

    println!("Server started successfully");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::REFERER,
            ])
            .supports_credentials();
        App::new()
            .wrap(
                actix_web::middleware::DefaultHeaders::new().add((header::REFERER, "*")),
            )
            .wrap(cors)
            .app_data(web::Data::new(AppState { db: pool.clone() }))
            .configure(user::handler::config)
            .wrap(Logger::default())
    })
    .bind(("0.0.0.0", 8081))?
    .run()
    .await
}
