use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;

mod routes;

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("devAPI is alive")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Could not connect to DB");

    storage::db::init_db(&pool).await.expect("DB init failed");

    let pool = Arc::new(pool);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(pool.clone()))
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive())
            .route("/health", web::get().to(health_check))
            .configure(routes::init)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
