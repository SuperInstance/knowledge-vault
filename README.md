# Knowledge Vault

[![crates.io](https://img.shields.io/crates/v/knowledge-vault)](https://crates.io/crates/knowledge-vault)
[![docs.rs](https://img.shields.io/docsrs/knowledge-vault)](https://docs.rs/knowledge-vault)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/SuperInstance/knowledge-vault/ci.yml)](https://github.com/SuperInstance/knowledge-vault/actions)

> High-performance vector database with semantic search for RAG applications

## ✨ Features

- **Vector Storage** - SQLite-based with vector similarity search
- **Smart Chunking** - Multiple strategies (code, markdown, sliding window)
- **Embedding Generation** - Pluggable embedders with async batch support
- **Semantic Search** - Fast vector similarity search with fallback
- **File Watching** - Automatic reindexing on file changes
- **Type Detection** - Automatic document classification

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
knowledge-vault = "0.1"
```

Index and search documents:

```rust
use knowledge_vault::{KnowledgeVault, DocumentIndexer, PlaceholderEmbedder};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open vault with 384-dimensional embeddings
    let vault = KnowledgeVault::open("knowledge.db", 384)?;

    // Create embedder
    let embedder = Arc::new(Mutex::new(PlaceholderEmbedder::new(384)));

    // Create indexer
    let (indexer, _handle) = DocumentIndexer::new(
        Arc::new(Mutex::new(vault)),
        embedder,
        Default::default(),
    );

    // Index a document
    indexer.index_file("README.md".into()).await?;

    Ok(())
}
```

## 📚 Examples

- **[Basic Indexing](examples/basic_indexing.rs)** - Create vault, index documents, search
- **[Code Search](examples/code_search.rs)** - Code-aware chunking for codebases
- **[Markdown Search](examples/markdown_search.rs)** - Heading-based chunking
- **[Streaming Index](examples/streaming_index.rs)** - Parallel indexing with channels
- **[With Privox](examples/with_privox.rs)** - PII redaction integration

Run examples:
```bash
cargo run --example basic_indexing
```

## 🏗️ Architecture

### Components

- **Vault** ([`KnowledgeVault`]) - SQLite storage with vector search
- **Chunker** ([`Chunker`]) - Splits documents into optimal pieces
- **Embeddings** ([`PlaceholderEmbedder`]) - Generates vector embeddings
- **Indexer** ([`DocumentIndexer`]) - Automates document ingestion
- **Watcher** ([`FileWatcher`]) - Monitors files for changes
- **Search** ([`VectorSearch`]) - Semantic similarity queries

### Chunking Strategies

| Strategy | Best For | Features |
|----------|----------|----------|
| **Code** | Source code | Function/class boundaries |
| **Markdown** | Documentation | Heading-based splitting |
| **Sliding Window** | Plain text | Overlapping chunks |

## 🔍 Vector Search

The vault supports two search modes:

1. **VSS (Virtual Table)** - Fast approximate nearest neighbor search
   - Requires SQLite-VSS extension
   - Best for large datasets (>10k chunks)

2. **Cosine Similarity** - Exact similarity calculation
   - Pure Rust implementation
   - Fallback when VSS unavailable
   - Suitable for smaller datasets

## 🔌 Custom Embedders

Implement the `EmbeddingProvider` trait:

```rust,ignore
use knowledge_vault::embeddings::EmbeddingProvider;
use async_trait::async_trait;

#[async_trait]
impl EmbeddingProvider for MyEmbedder {
    async fn embed(&self, text: &str) -> KnowledgeResult<Vec<f32>> {
        // Your embedding logic here
    }

    fn dimensions(&self) -> u32 {
        384
    }
}
```

## 📊 Performance

| Metric | Value |
|--------|-------|
| **Indexing Speed** | ~100 docs/sec (placeholder embeddings) |
| **Search Latency** | ~10ms for 1K chunks (cosine fallback) |
| **Storage** | ~1KB per chunk + embeddings |
| **Scaling** | Tested up to 100K documents |

## 🛠️ Advanced Usage

### Custom Chunking

```rust
use knowledge_vault::{Chunker, ChunkOptions};

let chunker = Chunker::with_options(ChunkOptions {
    chunk_size: 1024,
    chunk_overlap: 100,
    min_chunk_size: 200,
    respect_sentences: true,
    respect_paragraphs: true,
});

let chunks = chunker.chunk(document_content)?;
```

### File Watching

```rust,ignore
use knowledge_vault::{FileWatcher, WatchConfig};
use notify::RecursiveMode;

let config = WatchConfig {
    paths: vec!["./docs".into()],
    recursive: RecursiveMode::Recursive,
    ..Default::default()
};

let watcher = FileWatcher::new(config).await?;
watcher.start().await?;
```

## 🤝 Integration

### With Privox (PII Redaction)

```rust,ignore
use privox::PrivacyRedactor;
use knowledge_vault::{KnowledgeVault, Chunker};

// Redact PII before indexing
let redactor = PrivacyRedactor::new();
let redacted = redactor.redact(sensitive_content)?;

// Index redacted content
vault.add_document("private.txt", &redacted, "text")?;
```

See the [with_privox example](examples/with_privox.rs) for details.

## 📖 Documentation

- [API Documentation](https://docs.rs/knowledge-vault)
- [Examples](examples/)
- [Architecture Guide](docs/architecture.md) (coming soon)

## 🔬 Benchmarks

Run the benchmark suite:

```bash
cargo bench
```

## 🧪 Testing

Run all tests:

```bash
cargo test
```

## 📝 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

Built for the [SuperInstance](https://github.com/SuperInstance) ecosystem.

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

**Made with ❤️ by the SuperInstance team**
