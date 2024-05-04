// src/services.rs
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

// to do: change this based on the actual url
const DB_URL: &str = "http://localhost:8080";
static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

#[derive(Deserialize, Serialize, Debug)]
pub struct UploadResponse {
    id: i32,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct SearchResponse {
    result_1: String,
    result_2: String,
    result_3: String,
    result_4: String,
}

pub async fn upload(content: String) -> Result<UploadResponse, reqwest::Error> {
    println!("Content: {}", content);
    println!("IN upload RECIPE");
    let url = format!("{}/upload", DB_URL);
    let body = json!({
        "content": content
    });

    let response = CLIENT.post(url).json(&body).send().await?;
    let result: UploadResponse = response.json().await?;
    //let json_string = serde_json::to_string(&result);
    Ok(result)
}

pub async fn search(query: String, top_k: usize) -> Result<SearchResponse, reqwest::Error> {
    let url = format!("{}/search", DB_URL);
    let body = json!({
        "query": query,
        "top_k": top_k
    });

    let response = CLIENT.post(url).json(&body).send().await?;
    //let text = response.text().await?;

    // println!("text: {:?}", text);

    let result: SearchResponse = response.json().await?;
    //let json_string = serde_json::to_string(&result);
    Ok(result)

    // Ok(result)
}
