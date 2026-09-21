//! Markdown search example
//!
//! Demonstrates heading-aware chunking for markdown documents

use knowledge_vault::{
    embeddings::{DocType, EmbeddingPipeline},
    KnowledgeVault, PlaceholderEmbedder,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let vault = KnowledgeVault::open(PathBuf::from("docs_vault.db"), 384)?;
    let embedder = PlaceholderEmbedder::new(384);

    let markdown = r#"
# Getting Started

Welcome to the knowledge vault! This guide will help you get started.

## Installation

Add to your Cargo.toml:

```toml
[dependencies]
knowledge-vault = "0.1"
```

## Quick Start

```rust
use knowledge_vault::KnowledgeVault;

let vault = KnowledgeVault::open("vault.db", 384)?;
```

# Advanced Usage

## Custom Embedders

Implement the EmbeddingProvider trait for custom models.

## Chunking Strategies

Choose from:
- Code-aware chunking
- Markdown heading-based
- Sliding window for text

"#;

    // Chunk by markdown headings
    let pipeline = EmbeddingPipeline::new(PathBuf::from("/tmp/model"))?;
    let chunks = pipeline.chunk_document(markdown, DocType::Markdown);

    println!("✓ Chunked markdown into {} sections\n", chunks.len());

    // Display chunks with heading paths
    for (i, chunk) in chunks.iter().take(4).enumerate() {
        println!("--- Section {} ---", i + 1);
        if !chunk.metadata.heading_path.is_empty() {
            println!("Path: {}", chunk.metadata.heading_path.join(" > "));
        }
        println!(
            "Content: {}\n",
            chunk.content.chars().take(100).collect::<String>()
        );
    }

    // Index and search
    let doc_id = vault.add_document("guide.md", markdown, "markdown")?;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_id = format!("chunk_{}_{}", doc_id, i);
        vault.insert_chunk(&chunk_id, &doc_id, i as u32, &chunk.content, 0, 0, 0)?;

        let embedding = embedder.embed(&chunk.content).await?;
        vault.insert_embedding(&chunk_id, &embedding)?;
    }

    // Search
    let query = "How do I install the vault?";
    let query_emb = embedder.embed(query).await?;
    let results = vault.search(&query_emb, 2)?;

    println!("🔍 Query: '{}'\n", query);
    for (i, result) in results.iter().enumerate() {
        println!("{}. Score: {:.2}", i + 1, result.score);
        println!(
            "   {}\n",
            result.content.chars().take(100).collect::<String>()
        );
    }

    std::fs::remove_file("docs_vault.db")?;
    println!("✓ Done");

    Ok(())
}
