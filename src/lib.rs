//! # Knowledge Vault - Vector Database & Semantic Search
//!
//! A high-performance vector database for Retrieval-Augmented Generation (RAG) applications.
//! Provides document storage, intelligent chunking, embedding generation, and semantic search.
//!
//! ## Features
//!
//! - **Vector Storage**: SQLite-based with vector similarity search
//! - **Smart Chunking**: Multiple strategies (code, markdown, sliding window)
//! - **Embedding Generation**: Pluggable embedders with async batch support
//! - **Semantic Search**: Fast vector similarity search with fallback
//! - **File Watching**: Automatic reindexing on file changes
//! - **Type Detection**: Automatic document classification
//!
//! ## Architecture
//!
//! The vault system is built around several components:
//!
//! - **Vault** ([`KnowledgeVault`]): SQLite-based storage with vector similarity search
//! - **Chunker** ([`Chunker`]): Splits documents into optimal-sized chunks for embedding
//! - **Embeddings** ([`PlaceholderEmbedder`]): Generates vector embeddings for text
//! - **Indexer** ([`DocumentIndexer`]): Automates document ingestion and indexing
//! - **Watcher** ([`FileWatcher`]): Monitors files for changes and auto-reindexes
//! - **Search** ([`VectorSearch`]): Performs semantic similarity queries
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use knowledge_vault::{KnowledgeVault, DocumentIndexer, PlaceholderEmbedder};
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Open vault with 384-dimensional embeddings
//!     let vault = KnowledgeVault::open("knowledge.db", 384)?;
//!
//!     // Create embedder (placeholder for development)
//!     let embedder = std::sync::Arc::new(Mutex::new(PlaceholderEmbedder::new(384)));
//!
//!     // Create indexer
//!     let (indexer, _handle) = DocumentIndexer::new(
//!         std::sync::Arc::new(Mutex::new(vault)),
//!         embedder,
//!         Default::default(),
//!     );
//!
//!     // Index a document
//!     indexer.index_file("README.md".into()).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Document Storage
//!
//! Documents are stored with the following metadata:
//!
//! - **Content Hash**: SHA256 for deduplication
//! - **Chunks**: Document split into optimal pieces (default: 512 tokens)
//! - **Embeddings**: Vector representations for each chunk
//! - **Type Detection**: Automatic classification (code, markdown, text)
//!
//! ## Vector Search
//!
//! The vault supports two search modes:
//!
//! 1. **VSS (Virtual Table)**: Fast approximate nearest neighbor search
//!    - Requires SQLite-VSS extension
//!    - Best for large datasets (>10k chunks)
//!
//! 2. **Cosine Similarity**: Exact similarity calculation
//!    - Pure Rust implementation
//!    - Fallback when VSS unavailable
//!    - Suitable for smaller datasets
//!
//! ## Embedding Providers
//!
//! The crate provides a trait for pluggable embedding providers:
//!
//! ```rust,ignore
//! use knowledge_vault::embeddings::EmbeddingProvider;
//! use async_trait::async_trait;
//!
//! #[async_trait]
//! impl EmbeddingProvider for MyEmbedder {
//!     async fn embed(&self, text: &str) -> KnowledgeResult<Vec<f32>> {
//!         // Your embedding logic here
//!     }
//!
//!     fn dimensions(&self) -> u32 {
//!         384
//!     }
//! }
//! ```

pub mod chunker;
pub mod embeddings;
pub mod indexer;
pub mod search;
pub mod vault;
pub mod watcher;

pub use chunker::{Chunk, ChunkOptions, Chunker};
pub use embeddings::{EmbeddingProvider, LocalEmbedder, PlaceholderEmbedder};
pub use indexer::{DocumentIndexer, IndexCommand, IndexResult, IndexerConfig, IndexerHandle};
pub use search::{SearchOptions, SearchResult, VectorSearch};
pub use vault::{ChunkResult, Document, KnowledgeVault, VaultStats};
pub use watcher::{FileWatcher, WatchConfig};

/// Result type for knowledge operations
pub type KnowledgeResult<T> = std::result::Result<T, KnowledgeError>;

/// Knowledge error types
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Invalid document format: {0}")]
    InvalidFormat(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Watch error: {0}")]
    WatchError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
