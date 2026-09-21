//! Basic indexing example
//!
//! This example shows how to:
//! - Create a knowledge vault
//! - Index a document
//! - Search for similar content

use knowledge_vault::{ChunkOptions, Chunker, KnowledgeVault, PlaceholderEmbedder};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create or open vault
    let vault_path = "example_vault.db";
    let vault = KnowledgeVault::open(PathBuf::from(vault_path), 384)?;
    println!("✓ Opened vault at {}", vault_path);

    // Create embedder
    let embedder = PlaceholderEmbedder::new(384);
    println!("✓ Created placeholder embedder (384 dimensions)");

    // Sample document
    let content = r#"
# Knowledge Vault Guide

The knowledge vault is a vector database that stores documents as chunks.
Each chunk is embedded into a vector representation for semantic search.

## Features

- Document storage with deduplication
- Intelligent chunking strategies
- Vector similarity search
- File watching and auto-reindexing

## Usage

You can index files, search semantically, and build RAG applications.
"#;

    // Chunk the document
    let chunker = Chunker::with_options(ChunkOptions {
        chunk_size: 128,
        chunk_overlap: 20,
        min_chunk_size: 50,
        respect_sentences: true,
        respect_paragraphs: true,
    });

    let chunks = chunker.chunk(content)?;
    println!("✓ Created {} chunks", chunks.len());

    // Add document to vault
    let doc_id = vault.add_document("guide.md", content, "markdown")?;
    println!("✓ Added document with ID: {}", doc_id);

    // Insert chunks and embeddings
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_id = format!("chunk_{}_{}", doc_id, i);

        vault.insert_chunk(
            &chunk_id,
            &doc_id,
            chunk.index,
            &chunk.content,
            chunk.start_offset,
            chunk.end_offset,
            chunk.token_count,
        )?;

        let embedding = embedder.embed(&chunk.content).await?;
        vault.insert_embedding(&chunk_id, &embedding)?;
    }

    println!("✓ Inserted {} chunks with embeddings", chunks.len());

    // Search for similar content
    let query = "How does the vault store documents?";
    let query_embedding = embedder.embed(query).await?;
    let results = vault.search(&query_embedding, 3)?;

    println!("\n🔍 Search results for '{}':", query);
    for (i, result) in results.iter().enumerate() {
        println!("  {}. Score: {:.2}", i + 1, result.score);
        println!(
            "     Content: {}...\n",
            result.content.chars().take(80).collect::<String>()
        );
    }

    // Display stats
    let stats = vault.stats()?;
    println!("📊 Vault stats:");
    println!("  Documents: {}", stats.document_count);
    println!("  Chunks: {}", stats.chunk_count);
    println!("  Embeddings: {}", stats.embedding_count);

    // Cleanup
    std::fs::remove_file(vault_path)?;
    println!("\n✓ Cleaned up example database");

    Ok(())
}
