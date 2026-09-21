# Knowledge Vault — Archive Snapshot

> A slice-of-life artifact from the SuperInstance era

## What this is

A high-performance vector database library written in Rust, designed for RAG
(retrieval-augmented generation) applications. Provides SQLite-backed vector
storage with semantic search, smart chunking for code/markdown/sliding-window
inputs, pluggable embedders, file-watching-based automatic reindexing, and
automatic document type detection.

The crate exposes `KnowledgeVault`, `DocumentIndexer`, `PlaceholderEmbedder`
and friends — see `examples/basic_indexing.rs` for the smallest end-to-end
walk-through.

## What it was good at

- **SQLite + rusqlite** rather than a separate vector DB process — single-file,
  zero-ops storage that you can `scp` around.
- **Multiple chunking strategies** out of the box: code-aware, markdown-aware,
  and a sliding-window fallback. See `src/chunker.rs`.
- **Pluggable embedding interface** — bring your own model, use the included
  `PlaceholderEmbedder` for tests, or wire up a remote embedder.
- **File watcher** (`src/watcher.rs`) reindexes on change without a daemon.
- **Optional `vss` feature** for SQLite-VSS when the extension is available.

## Why we moved on

We were building this in parallel with several other RAG back-ends in 2025–2026
and converged on a different storage layer (one that used a hosted vector index
rather than embedded SQLite). The mechanism here — the chunker strategies and
the embedder-pluggability — got folded into the next iteration in a different
shape, so the standalone crate was retired rather than maintained.

But the chunker logic and the embedder-trait abstraction are still interesting
on their own. Anyone building a small, self-contained RAG pipeline without
wanting to spin up Qdrant / Weaviate / Pinecone could grab this and run.

## How to use it

```toml
[dependencies]
knowledge-vault = { git = "https://github.com/SuperInstance/knowledge-vault" }
tokio = { version = "1", features = ["full"] }
```

```rust
use knowledge_vault::{KnowledgeVault, DocumentIndexer, PlaceholderEmbedder};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vault = KnowledgeVault::open("knowledge.db", 384)?;
    let embedder = Arc::new(Mutex::new(PlaceholderEmbedder::new(384)));
    let indexer = DocumentIndexer::new(vault, embedder);
    // ...
    Ok(())
}
```

Run the examples with `cargo run --example basic_indexing` etc.

## How to get the big pieces

Nothing external is required to build this — it's pure Rust with `rusqlite`
binding to whatever SQLite you have installed. If you want the optional
SQLite-VSS extension, enable the `vss` feature (`cargo build --features vss`).

No model weights, no external services, no datasets.

## Notable artifacts in this snapshot

- `src/vault.rs` + `src/vault_fixed.rs` — both kept; `vault_fixed.rs` is the
  later iteration that resolved a locking bug in the original.
- `examples/with_privox.rs` — wiring example that shows using this crate
  alongside another SuperInstance component (`privox`). That sibling crate
  has its own archive snapshot if you're curious.
- `PUBLISHING_CHECKLIST.md` — preserved as-is from the pre-publish
  documentation pass.

## License

Dual-licensed under MIT or Apache-2.0, at your option. See `LICENSE-MIT` and
`LICENSE-APACHE`.

## A note from the archiver

Pushed by the SuperInstance archive curator on 2026-09-21. The on-disk
directory at the time of archival was ~914 MB; the overwhelming majority of
that was `target/` build artifacts and a fat `.git/` object store. The
repository published here contains only the source tree, examples, CI config,
and documentation — the build outputs are intentionally not part of the
public history.

If you're forking this and want the original `target/` to investigate
compiler caches, you'll need to recompile from scratch with `cargo build`.

— *Archive Curator, slice-of-life preservation*
