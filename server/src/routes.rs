// src/routes.rs
use crate::services::{search, upload};
use actix_web::web::Query;
use actix_web::Responder;
use actix_web::{web, HttpResponse};
use serde::Deserialize;

// Define a struct for the expected query parameters
#[derive(Deserialize)]
pub struct QueryParams {
    query: String,
    top_k: usize,
}

#[derive(Deserialize)]
pub struct UploadParams {
    content: String,
}

// Update the function signature to use the struct
pub async fn upload_document(params: web::Json<UploadParams>) -> impl Responder {
    // Use params.content and params.top_k as needed in your function
    println!("IN upload_document");
    match upload(params.content.clone()).await {
        Ok(recipe) => HttpResponse::Ok().json(recipe),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn search_documents(params: web::Query<QueryParams>) -> HttpResponse {
    let inner_params = params.into_inner();
    match search(inner_params.query, inner_params.top_k).await {
        Ok(recipes) => HttpResponse::Ok().json(recipes),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
