// src/routes.rs
use actix_web::{web, HttpResponse};
use crate::services::{upload, search};
use serde::Deserialize;
use actix_web::Responder;

// Define a struct for the expected query parameters
#[derive(Deserialize)]
pub struct QueryParams {
    query: String,
    top_k: usize,
}

// Update the function signature to use the struct
pub async fn upload_document(params: web::Query<String>) -> impl Responder {
    // Use params.content and params.top_k as needed in your function
    println!("IN upload_document");
    match upload(params.clone()).await {
        Ok(recipe) => HttpResponse::Ok().json(recipe),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn search_documents(params: web::Query<QueryParams>) -> HttpResponse {
    match search(params.query.clone().into_inner(), , query.top_k).await {
        Ok(recipes) => HttpResponse::Ok().json(recipes),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}