//! Code search example
//!
//! This example demonstrates code-aware chunking and search

use knowledge_vault::{
    embeddings::{DocType, EmbeddingPipeline},
    Chunker, KnowledgeVault, PlaceholderEmbedder,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let vault = KnowledgeVault::open(PathBuf::from("code_vault.db"), 384)?;
    println!("✓ Opened code vault");

    // Sample Rust code
    let code = r#"
pub struct VectorStore {
    data: Vec<Vec<f32>>,
    dimension: usize,
}

impl VectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            data: Vec::new(),
            dimension,
        }
    }

    pub fn add(&mut self, embedding: Vec<f32>) -> Result<(), String> {
        if embedding.len() != self.dimension {
            return Err("Dimension mismatch".to_string());
        }
        self.data.push(embedding);
        Ok(())
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<usize> {
        // Calculate similarities
        let mut similarities: Vec<(usize, f32)> = self.data
            .iter()
            .enumerate()
            .map(|(i, emb)| (i, cosine_similarity(query, emb)))
            .collect();

        // Sort by similarity
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Return top k
        similarities.into_iter().take(k).map(|(i, _)| i).collect()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
"#;

    // Use document chunker for code-aware splitting
    let pipeline = EmbeddingPipeline::new(PathBuf::from("/tmp/model"))?;

    // Chunk as code
    let chunks = pipeline.chunk_document(code, DocType::Code);
    println!("✓ Chunked code into {} pieces", chunks.len());

    // Display chunks
    for (i, chunk) in chunks.iter().take(3).enumerate() {
        println!("\n--- Chunk {} ---", i + 1);
        println!("Language: {:?}", chunk.metadata.language);
        println!("Content:\n{}", chunk.content);
    }

    std::fs::remove_file("code_vault.db")?;
    println!("\n✓ Cleaned up");

    Ok(())
}
