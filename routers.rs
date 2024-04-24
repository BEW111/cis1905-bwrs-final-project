use actix_web::{web, App, HttpServer, HttpResponse, Responder, HttpRequest};
use std::sync::Mutex;

// Handler for GET request
async fn get_message(data: web::Data<AppState>) -> impl Responder {
    let message = data.message.lock().unwrap();
    HttpResponse::Ok().body(message.clone())
}

// Handler for POST request
async fn set_message(req_body: String, data: web::Data<AppState>) -> impl Responder {
    let mut message = data.message.lock().unwrap();
    *message = req_body;
    HttpResponse::Ok().body("Message updated")
}