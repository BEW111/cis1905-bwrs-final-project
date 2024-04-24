use actix_web::{web, App, HttpServer, Responder};

async fn index() -> impl Responder {
    "Hello world!"
}

struct AppState {
    app_name: String,
    message: Mutex<String>,
}

#[get("/")]
async fn index(data: web::Data<AppState>) -> String {
    let app_name = &data.app_name; // <- get app_name
    format!("Hello {app_name}!") // <- response with app_name
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Chef Engine"),
            })).route("/get_message", web::get().to(get_message))
            // Route for POST request
            .route("/set_message", web::post().to(set_message))
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
