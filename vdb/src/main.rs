use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{json, map::Map, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use num_traits::{Float, Num, NumCast};
use std::ops::{Add, Mul};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use anyhow::{Result, Context};



#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: u32,
    content: String,
    embedding: Vec<f32>,
}

struct Database {
    documents: HashMap<u32, Document>,
    next_id: u32, // keeps track of the id for the document that would be added next
}

#[derive(Serialize, Deserialize)]
struct SearchQuery {
    top_k: usize,
    query: String,
}

#[derive(Serialize, Deserialize)]
struct UploadQuery {
    content: String
}

// If a type implements this trait, it can be compared with another type
// to rank its similarity to that type
pub trait Similarity<S> {
    fn dot_product(&self, other: &Self) -> S;
    fn l2_norm(&self) -> S;
    fn similarity(&self, other: &Self) -> S;
}

impl<T> Similarity<f32> for Vec<T>
where
    T: Num + Copy + NumCast,
{
    fn dot_product(&self, other: &Self) -> f32 {
        // Use iterator zipping to combine the two vectors element-wise
        self.iter()
            .zip(other)
            .map(|(&a, &b)| NumCast::from(a * b).unwrap_or(0.0_f32))
            .sum()
    }

    fn l2_norm(&self) -> f32 {
        // Map each element to its square, cast to f32, sum, and sqrt
        self.iter()
            .map(|&a| {
                let squared = a * a;
                NumCast::from(squared).unwrap_or(0.0_f32)
            })
            .sum::<f32>()
            .sqrt()
    }

    fn similarity(&self, other: &Self) -> f32 {
        let dot_product = self.dot_product(other);
        let self_norm = self.l2_norm();
        let other_norm = other.l2_norm();

        // Handle potential division by zero
        if self_norm * other_norm == 0.0 {
            0.0
        } else {
            dot_product / (self_norm * other_norm)
        }
    }
}

// Calculates the cosine similarity between two vectors
// TODO: make this into a trait
fn cosine_similarity(a: &Vec<f32>, b: &Vec<f32>) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    dot_product / (norm_a * norm_b)
}

fn generate_embedding(document_content: &str) -> Result<Vec<f32>, anyhow::Error> {
    // Call to external API to generate embedding
    let mut model = TextEmbedding::try_new(InitOptions {
        model_name: EmbeddingModel::AllMiniLML6V2,
        show_download_progress: true,
        ..Default::default()
    })?;

    let embedding = model.embed(vec![document_content], None)?; 

   Ok(embedding[0].clone())
}

impl Database {
    // Inserts a new document into the database, given the string content and the embedding
    fn insert(&mut self, content: String, embedding: Vec<f32>) -> u32 {
        let doc = Document {
            id: self.next_id,
            content,
            embedding,
        };
        self.documents.insert(self.next_id, doc);
        self.next_id += 1;
        self.next_id - 1
    }

    // Given a query embedding, finds the document nearest to this
    fn search(&self, query: &str, top_k: usize) -> Option<Vec<&Document>> {
        let query_embedding_result = generate_embedding(query);
        let mut query_embedding : Vec<f32> = Vec::new();

        match query_embedding_result {
            Ok(ref embedding) => {
                query_embedding = embedding.clone();
                // Do something with the similarity
            },
            Err(e) => {
                // Handle the error, e.g., logging or setting a default value
                println!("Failed to calculate embedding: {}", e);
            }
        }

        if top_k == 0 || self.documents.is_empty() {
            return None;
        }

        println!("Searching for query: {:?}", query);
        
        let mut all_doc_similarites : Vec<(&Document, f32)> = self.documents.values().map(|doc| {
            let similarity = doc.embedding.similarity(&query_embedding);
            (doc, similarity)
        }).collect();

        all_doc_similarites.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_k_docs : Vec<&Document> = all_doc_similarites.into_iter().take(top_k).map(|(doc, _)| doc).collect();

        println!("Results: {:?}", top_k_docs);

        if top_k_docs.is_empty() {
            None
        } else {
            Some(top_k_docs)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = web::Data::new(Mutex::new(Database { documents: HashMap::new(), next_id: 0 }));
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(hello)
            .route("/upload", web::post().to(upload_document))
            .route("/search", web::post().to(search_document))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn upload_document(data: web::Data<Mutex<Database>>, json: web::Json<UploadQuery>) -> impl Responder {
    println!("got upload_document");
    let mut db = data.lock().unwrap(); // Accessing the database safely
    // Assume generate_embedding is synchronous just for placeholder; you'd use .await for async
    let json_embedding = generate_embedding(&json.content);
    let mut content_embedding : Vec<f32> = Vec::new();

    // let mut doc_id = 0;
    // if let Ok(embedding) = generate_embedding(&json.content) {
    //     println!("Embedding generated successfully");
    //     doc_id = db.insert(json.content.clone(), embedding);
    //     // Further processing
    // } else {
    //     // Log error or handle it appropriately
    //     println!("Failed to generate embedding");
    // }

    match json_embedding {
        Ok(ref embedding) => {
            content_embedding = embedding.clone();
        },
        Err(e) => {
            // Handle the error, e.g., logging or setting a default value
            println!("Failed to calculate embedding: {}", e);
        }
    }
    let doc_id = db.insert(json.content.clone(), content_embedding);

    println!("Document inserted with id: {}", doc_id);
    let response_json = json!({ "id": doc_id });
    HttpResponse::Ok().json(response_json)
}

fn doc_vec_to_json(doc_list: Vec<&Document>) -> Value {
    let mut json_map = Map::new();

    for (index, doc) in doc_list.iter().enumerate() {
        let doc_content = &doc.content;
        let key = format!("result_{}", index + 1);
        json_map.insert(key, Value::String(doc_content.to_string()));
    }

    // Convert map to Value
    Value::Object(json_map)
}

async fn search_document(data: web::Data<Mutex<Database>>, json: web::Json<SearchQuery>) -> impl Responder {
    let db = data.lock().unwrap();
    if let Some(docs) = db.search(&json.query.clone(), json.top_k.clone()) {
        HttpResponse::Ok().json(doc_vec_to_json(docs))
    } else {
        HttpResponse::NotFound().finish()
    }
}
