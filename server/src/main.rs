// src/main.rs
use actix_web::{web, App, HttpServer};

mod routes;
mod services;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/hello", web::get().to(|| async { "Hello, world!" }))
            .route(
                "/api/upload_document",
                web::post().to(routes::upload_document),
            )
            .route(
                "/api/search_documents",
                web::post().to(routes::search_documents),
            )
    })
    .bind("127.0.0.1:3535")?
    .run()
    .await
}
