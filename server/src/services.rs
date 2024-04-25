// src/services.rs
use crate::models::Recipe;
use reqwest::Client;


// to do: change this based on the actual url
const DB_URL : &str = "http://localhost:8080/";


pub async fn upload(content: String) -> Result<String, reqwest::Error> {
    println!("IN upload RECIPE");
    let client = Client::new();
    client.post(format!("{}/upload?content={}", DB_URL, content))
          .send().await?
          .json::<String>().await
}


pub async fn search(query: String) -> Result<Vec<String>, reqwest::Error> {
    let client = Client::new();
    client.get(format!("{}/search?query={}", DB_URL, query))
          .send().await?
          .json::<Vec<String>>().await
}