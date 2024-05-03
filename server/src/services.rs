// src/services.rs
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::json;

// to do: change this based on the actual url
const DB_URL: &str = "http://localhost:8080/";
static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

pub async fn upload(content: String) -> Result<String, reqwest::Error> {
    println!("Content: {}", content);
    println!("IN upload RECIPE");
    let url = format!("{}/upload", DB_URL);
    let body = json!({
        "content": content
    });

    CLIENT
        .post(url)
        .json(&body)
        .send()
        .await?
        .json::<String>()
        .await
}

pub async fn search(query: String, top_k: usize) -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/search", DB_URL);
    let body = json!({
        "query": query,
        "top_k": top_k
    });

    CLIENT
        .post(url)
        .json(&body)
        .send()
        .await?
        .json::<Vec<String>>()
        .await
}
