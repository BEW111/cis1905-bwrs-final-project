use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use num_traits::{Float, Num, NumCast};
use serde::{Deserialize, Serialize};
use serde_json::{json, map::Map, Value};

use actix_rt::time::interval;
use std::time::Duration;

use std::cmp::Ordering;
use std::ops::{Add, Mul};
use std::sync::{Arc, Mutex};

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::{BinaryHeap, VecDeque};

use rand::prelude::*;
use rand_distr::Uniform;
use std::f64::consts::E;

// for a binary heap with floats
// https://stackoverflow.com/questions/39949939/how-can-i-implement-a-min-heap-of-f64-with-rusts-binaryheap
#[derive(PartialEq)]
struct MinNonNan(f32);

impl Eq for MinNonNan {}

impl PartialOrd for MinNonNan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for MinNonNan {
    fn cmp(&self, other: &MinNonNan) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => Ordering::Less,
        }
    }
}

// graph data structure
#[derive(Clone)]
struct GraphLayer {
    pub entry: Option<u32>,
    pub adjacency: HashMap<u32, Vec<u32>>, // map from node id to list of adjacent node ids
}

impl GraphLayer {
    fn new(entry: Option<u32>) -> Self {
        GraphLayer {
            entry,
            adjacency: HashMap::new(),
        }
    }

    pub fn set_entry_node(&mut self, entry: u32) -> Result<()> {
        self.entry = Some(entry);

        Ok(())
    }

    pub fn add_node(&mut self, node_id: u32) -> Result<()> {
        if let Entry::Vacant(entry) = self.adjacency.entry(node_id) {
            let neighbors: Vec<u32> = Vec::new();
            entry.insert(neighbors);
        }

        Ok(())
    }

    pub fn add_neighbor(&mut self, node_id: u32, neighbor_id: u32) -> Result<()> {
        if self.adjacency.contains_key(&node_id) {
            match self.adjacency.get_mut(&node_id) {
                Some(neighbors) => {
                    if !neighbors.contains(&neighbor_id) {
                        neighbors.push(neighbor_id);
                    }
                }
                None => {
                    println!("Error getting neighbors for node {:?}", node_id);
                }
            }
        } else {
            self.adjacency.insert(node_id, vec![neighbor_id]);
        }

        if self.adjacency.contains_key(&neighbor_id) {
            match self.adjacency.get_mut(&neighbor_id) {
                Some(neighbors) => {
                    if !neighbors.contains(&node_id) {
                        neighbors.push(node_id);
                    }
                }
                None => {
                    println!("Error getting neighbors for node {:?}", neighbor_id);
                }
            }
        } else {
            self.adjacency.insert(neighbor_id, vec![node_id]);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    id: u32,
    content: String,
    embedding: Vec<f32>,
}

struct Database {
    documents: HashMap<u32, Document>,
    next_id: u32, // keeps track of the id for the document that would be added next
    num_layers: usize,
    graph_layers: Vec<GraphLayer>,
}

#[derive(Serialize, Deserialize)]
struct SearchQuery {
    top_k: usize,
    query: String,
}

#[derive(Serialize, Deserialize)]
struct UploadQuery {
    content: String,
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

pub trait Embeddable {
    fn generate_embedding(&self) -> Result<Vec<Vec<f32>>>;
}

impl Embeddable for str {
    fn generate_embedding(&self) -> Result<Vec<Vec<f32>>> {
        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: true,
            ..Default::default()
        })?;

        let embedding = model.embed(vec![self], None)?;

        Ok(embedding.clone())
    }
}
/// Implementing the trait for the `String` type.
impl Embeddable for String {
    fn generate_embedding(&self) -> Result<Vec<Vec<f32>>> {
        self.as_str().generate_embedding()
    }
}

/// Implementing the trait for a vector of Strings
impl Embeddable for Vec<String> {
    fn generate_embedding(&self) -> Result<Vec<Vec<f32>>> {
        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: true,
            ..Default::default()
        })?;

        let embedding = model.embed(self.to_vec(), None)?;

        Ok(embedding.clone())
    }
}

impl Database {
    fn new(num_layers: usize) -> Self {
        let mut graphs: Vec<GraphLayer> = Vec::new();
        for _ in 0..num_layers {
            graphs.push(GraphLayer::new(None));
        }

        Database {
            documents: HashMap::new(),
            next_id: 0,
            num_layers: num_layers,
            graph_layers: graphs,
        }
    }

    fn generate_level(&mut self, assign_probas: &[f64], rng: &mut ThreadRng) -> usize {
        // Create a uniform distribution from 0.0 to 1.0
        let between = Uniform::from(0.0..1.0);
        let f = rng.sample(between);
        let mut cumulative_probability = 0.0;

        for (level, &probability) in assign_probas.iter().enumerate() {
            cumulative_probability += probability;
            if f < cumulative_probability {
                return level;
            }
        }

        // Return the last level in the unlikely event that none are selected
        assign_probas.len() - 1
    }

    fn set_assign_probas(&mut self, m: usize, m_l: f64) -> (Vec<f64>, Vec<usize>) {
        let mut nn = 0; // Set nearest neighbors count = 0
        let mut cum_nneighbor_per_level = Vec::new();
        let mut level = 0; // We start at level 0
        let mut assign_probas = Vec::new();

        loop {
            let proba = E.powf(-(level as f64) / m_l) * (1.0 - E.powf(-1.0 / m_l));
            // Once we reach low prob threshold, we've created enough levels
            if proba < 1e-9 {
                break;
            }
            assign_probas.push(proba);

            // Calculate the number of neighbors for this level
            nn += (2 - level / (self.num_layers - 1)) * m;
            cum_nneighbor_per_level.push(nn);
            level += 1;
        }
        (assign_probas, cum_nneighbor_per_level)
    }

    // Inserts a new document into the database, given the string content and the embedding
    fn insert(&mut self, content: String, embedding: Vec<f32>) -> u32 {
        let doc_id = self.next_id;

        // New document to be inserted
        let doc = Document {
            id: doc_id,
            content,
            embedding: embedding.clone(),
        };

        // For each layer from here to 0, we'll find the M nearest neighbors and add links
        //let (assign_probas, cum_nneighbor_per_level) = self.set_assign_probas(3, 0.07);

        let assign_probas = vec![0.5, 0.3, 0.15, 0.05];
        let cun_nneighbor_per_level = vec![4, 3, 2, 1];
        let mut rng = rand::thread_rng();

        let mut l = self.generate_level(&assign_probas, &mut rng); // highest level to insert this node at
        let graph_is_empty = self.documents.len() == 0;
        if graph_is_empty {
            // this node must be inserted at the top level if nothing is in the graph yet
            l = self.num_layers - 1;

            // make it the entrypoint
            match self.graph_layers.get_mut(l) {
                Some(top_layer) => {
                    match top_layer.set_entry_node(doc_id) {
                        Ok(_) => (),
                        Err(e) => println!("Error setting: {}", e),
                    };
                }
                None => println!("Error getting top layer"),
            }
        }
        println!("node will be inserted at layer {:?}", l);

        for curr_layer in 0..(l + 1) {
            let m = cun_nneighbor_per_level[curr_layer]; // number of neighbors to connect to
            println!("Number of links in layer {:?}: {:?}", curr_layer, m);

            if let Some(curr_graph) = self.graph_layers.get_mut(curr_layer) {
                // Add the node to this graph
                match curr_graph.add_node(doc_id) {
                    Ok(_) => (),
                    Err(e) => println!("Error adding neighbor: {}", e),
                };

                let embedding_clone = embedding.clone();
                let graph_clone = curr_graph.clone();
                // Find the M nearest neighbors, and add all of these edges
                let mut all_doc_similarities: Vec<(&u32, f32)> = graph_clone
                    .adjacency
                    .keys()
                    .map(|other_doc_id| {
                        if let Some(other_doc) = self.documents.get(other_doc_id) {
                            let other_embedding = &other_doc.embedding;
                            Some((other_doc_id, embedding_clone.similarity(other_embedding)))
                        } else {
                            None
                        }
                    })
                    .flatten() // filter out None values
                    .collect();

                // Sort similarities
                all_doc_similarities
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                // Get the top m
                let top_m_docs: Vec<&u32> = all_doc_similarities
                    .into_iter()
                    .take(m)
                    .map(|(doc, _)| doc)
                    .collect();

                for new_neighbor_id in top_m_docs {
                    println!(
                        "Adding link from node {:?} to node {:?}",
                        doc_id, new_neighbor_id
                    );
                    match curr_graph.add_neighbor(doc_id, *new_neighbor_id) {
                        Ok(_) => (),
                        Err(e) => println!("Error adding neighbor: {}", e),
                    }
                }
            }
        }

        // Record the document
        self.documents.insert(self.next_id, doc);
        self.next_id += 1;
        self.next_id - 1
    }

    fn search(&mut self, query: &str, top_k: usize) -> Option<Vec<&Document>> {
        println!("Query is {:?}", query);
        let query_embedding_result = query.generate_embedding();
        let mut query_embedding: Vec<f32> = Vec::new();

        match query_embedding_result {
            Ok(ref embedding) => {
                query_embedding = embedding[0].clone();
                // Do something with the similarity
            }
            Err(e) => {
                // Handle the error, e.g., logging or setting a default value
                println!("Failed to calculate embedding: {}", e);
            }
        }

        if top_k == 0 || self.documents.is_empty() {
            return None;
        }

        let mut curr_entry_node: u32 = u32::MAX;
        let mut curr_node: u32 = u32::MAX;

        // for keeping track of the most similar nodes
        let mut closest_docs = BinaryHeap::new(); // (similarity, doc_id)
        let mut visited_docs = HashSet::new();

        for curr_layer in (0..self.num_layers).rev() {
            println!("Currently at layer {:?}", curr_layer);
            let curr_graph = match self.graph_layers.get(curr_layer) {
                Some(graph) => graph,
                None => {
                    println!("Error getting graph at layer {:?}", curr_layer);
                    continue;
                }
            };
            if let Some(entry) = curr_graph.entry {
                curr_entry_node = entry;
            }

            curr_node = curr_entry_node;
            println!("Starting search at node {:?}", curr_node);
            let mut best_similarity: f32 = self
                .documents
                .get(&curr_node)
                .unwrap()
                .embedding
                .similarity(&query_embedding);
            if !visited_docs.contains(&curr_node) {
                if closest_docs.len() >= top_k {
                    closest_docs.pop();
                }
                closest_docs.push((MinNonNan(best_similarity), curr_node));
                visited_docs.insert(curr_node);
            }

            loop {
                // 1. get all neighbors of this current node
                // 2. compare the vector embedding of the doc to be inserted
                //    against all neighbors
                let mut neighbor_similarities: Vec<(&u32, f32)> = curr_graph
                    .adjacency
                    .get(&curr_node)
                    .unwrap()
                    .iter()
                    .map(|doc_id| {
                        let doc = self.documents.get(doc_id).unwrap();
                        let similarity = doc.embedding.similarity(&query_embedding);
                        println!(
                            "Similarity between query and doc {:?} is {:?}",
                            doc.content, similarity
                        );

                        // keep track of the closest nodes
                        if !visited_docs.contains(doc_id) {
                            if closest_docs.len() >= top_k {
                                closest_docs.pop();
                            }
                            closest_docs.push((MinNonNan(similarity), *doc_id));
                            visited_docs.insert(*doc_id);
                        }
                        (doc_id, similarity)
                    })
                    .collect();

                neighbor_similarities
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                if neighbor_similarities.len() == 0 {
                    curr_entry_node = curr_node;
                    println!(
                        "node {:?} is the best for this layer, moving down",
                        curr_node
                    );
                    break;
                }

                // if none of the neighbor similarities are higher than
                // best similarity, then drop down to the next layer
                if neighbor_similarities[0].1 <= best_similarity {
                    curr_entry_node = curr_node;
                    println!(
                        "node {:?} is the best for this layer, moving down",
                        curr_node
                    );
                    break;
                }

                // otherwise, set current node to the neighbor with the best similarity
                if neighbor_similarities[0].1 > best_similarity {
                    curr_node = *neighbor_similarities[0].0;
                    best_similarity = neighbor_similarities[0].1;
                }
            }
        }

        // at this point, we're at some current node with the document id we want,
        // so we can simply return the corresponding document
        let closest_docs_vec: Vec<&Document> = closest_docs
            .into_iter()
            .map(|t| self.documents.get(&t.1).unwrap())
            .collect();
        Some(closest_docs_vec)
    }
}

struct AppState {
    database: Mutex<Database>,
    upload_queue: Mutex<VecDeque<String>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // We want data to be shared state between threads, so create it outside HttpServer
    let data = web::Data::new({
        AppState {
            database: Mutex::new(Database::new(4)),
            upload_queue: Mutex::new(VecDeque::new()),
        }
    });

    // Spawn a future
    let data_clone = data.clone();
    actix_rt::spawn(async move {
        batch_process(data_clone).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/upload", web::post().to(upload_document))
            .route("/upload_old", web::post().to(upload_document_old))
            .route("/search", web::post().to(search_document))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// Batching process
async fn batch_process(data: web::Data<AppState>) {
    let mut interval = interval(Duration::from_millis(500));

    loop {
        interval.tick().await;
        let mut documents = Vec::new();

        {
            let mut queue = data.upload_queue.lock().unwrap();
            if queue.is_empty() {
                continue;
            }

            while let Some(doc) = queue.pop_front() {
                documents.push(doc);
            }
        }

        if !documents.is_empty() {
            match documents.clone().generate_embedding() {
                Ok(embeddings) => {
                    for (embedding, content) in embeddings.into_iter().zip(documents) {
                        let doc_id = data
                            .database
                            .lock()
                            .unwrap()
                            .insert(content.to_string(), embedding);
                        println!("Document inserted with id: {}", doc_id);
                    }
                }
                Err(_) => (),
            }
        }
    }
}

async fn upload_document(
    data: web::Data<AppState>,
    json: web::Json<UploadQuery>,
) -> impl Responder {
    let mut queue = data.upload_queue.lock().unwrap();
    queue.push_back(json.content.clone());
    HttpResponse::Accepted().json("Document received and will be processed.")
}

async fn upload_document_old(
    data: web::Data<AppState>,
    json: web::Json<UploadQuery>,
) -> impl Responder {
    println!("got upload_document");

    let mut db = match data.database.lock() {
        Ok(db) => db,
        Err(poisoned) => {
            println!("Error getting lock: {:?}", poisoned);
            return HttpResponse::InternalServerError().finish();
        }
    }; // Assume generate_embedding is synchronous just for placeholder; you'd use .await for async
    let json_embedding = json.content.generate_embedding();
    let mut content_embedding: Vec<f32> = Vec::new();

    match json_embedding {
        Ok(ref embedding) => {
            content_embedding = embedding[0].clone();
        }
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

async fn search_document(
    data: web::Data<AppState>,
    json: web::Json<SearchQuery>,
) -> impl Responder {
    let mut db = match data.database.lock() {
        Ok(db) => db,
        Err(poisoned) => {
            println!("Error getting lock: {:?}", poisoned);
            return HttpResponse::InternalServerError().finish();
        }
    };
    if let Some(doc_list) = db.search(&json.query.clone(), json.top_k.clone()) {
        // HttpResponse::Ok().json(json! ({"id": doc.id, "content": doc.content}))
        HttpResponse::Ok().json(doc_vec_to_json(doc_list))
    } else {
        HttpResponse::NotFound().finish()
    }
}
