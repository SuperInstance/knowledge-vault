//! Streaming index example
//!
//! Shows how to index documents in parallel using the channel-based indexer

use knowledge_vault::{DocumentIndexer, IndexerConfig, KnowledgeVault, PlaceholderEmbedder};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create vault
    let vault = Arc::new(Mutex::new(KnowledgeVault::open("streaming_vault.db", 384)?));

    // Create embedder
    let embedder = Arc::new(Mutex::new(PlaceholderEmbedder::new(384)));

    // Create indexer with channel
    let config = IndexerConfig {
        skip_duplicates: true,
        ..Default::default()
    };

    let (indexer, _handle) = DocumentIndexer::new(vault, embedder, config);
    println!("✓ Created channel-based indexer");

    // Create sample documents
    let docs = vec![
        (
            "doc1.txt",
            "This is the first document about AI and machine learning.",
        ),
        (
            "doc2.txt",
            "Machine learning models require large datasets for training.",
        ),
        (
            "doc3.txt",
            "AI systems can process natural language effectively.",
        ),
    ];

    // Index all documents concurrently
    let mut tasks = Vec::new();
    for (filename, content) in docs {
        let idxer = indexer.clone();
        let task = tokio::spawn(async move {
            idxer
                .index_content(
                    content.to_string(),
                    filename.to_string(),
                    "text".to_string(),
                    None,
                )
                .await
        });
        tasks.push(task);
    }

    // Wait for all indexing to complete
    for task in tasks {
        task.await??;
    }

    println!("✓ Indexed all documents");

    // Get stats
    let vault = KnowledgeVault::open("streaming_vault.db", 384)?;
    let stats = vault.stats()?;
    println!("\n📊 Stats:");
    println!("  Documents: {}", stats.document_count);
    println!("  Chunks: {}", stats.chunk_count);

    std::fs::remove_file("streaming_vault.db")?;
    println!("\n✓ Cleaned up");

    Ok(())
}
