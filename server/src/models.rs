// src/models.rs
use serde::{Deserialize, Serialize};

// DEPRECATED
// To do change this based on the actual data model
#[derive(Serialize, Deserialize)]
pub struct Recipe {
    pub id: i32,
    pub title: String,
    pub ingredients: Vec<String>,
    pub instructions: String,
}
