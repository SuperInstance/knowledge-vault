//! Knowledge Vault
//!
//! SQLite-based storage for documents, chunks, and embeddings.
//! Uses sqlite-vss for vector similarity search.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::{KnowledgeError, KnowledgeResult};

/// A document in the knowledge vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID
    pub id: String,
    /// Original file path
    pub path: Option<String>,
    /// Document title
    pub title: String,
    /// Document type (markdown, text, code, etc.)
    pub doc_type: String,
    /// SHA256 hash of content for deduplication
    pub content_hash: String,
    /// Number of chunks
    pub chunk_count: u32,
    /// Original content size in bytes
    pub size_bytes: u64,
    /// When indexed
    pub indexed_at: chrono::DateTime<chrono::Utc>,
    /// Last modified
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Vault statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub document_count: u64,
    pub chunk_count: u64,
    pub embedding_count: u64,
    pub total_size_bytes: u64,
    pub database_size_bytes: u64,
    pub embedding_dimensions: u32,
}

/// Result from a vector similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    pub chunk_id: String,
    pub document_id: String,
    pub content: String,
    pub document_title: String,
    pub document_path: Option<String>,
    pub score: f32,
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Knowledge vault backed by SQLite-VSS
///
/// Thread-safe: Uses Arc<Mutex<Connection>> internally to support
/// concurrent access from multiple threads.
#[derive(Clone)]
pub struct KnowledgeVault {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    embedding_dimensions: u32,
}

impl KnowledgeVault {
    /// Helper to execute a function with the database lock held
    fn with_conn<F, R>(&self, f: F) -> KnowledgeResult<R>
    where
        F: FnOnce(&rusqlite::Connection) -> Result<R, rusqlite::Error>,
    {
        let conn = self.conn.lock()
            .map_err(|e| KnowledgeError::Internal(format!("Lock poisoned: {}", e)))?;
        f(&*conn).map_err(|e| KnowledgeError::DatabaseError(e.to_string()))
    }

    /// Execute a statement that doesn't return a connection-bound value
    fn execute_sql(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> KnowledgeResult<()> {
        self.with_conn(|conn| conn.execute(sql, params))
    }

    /// Query and return a single value
    fn query_row<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], f: F) -> KnowledgeResult<T>
    where
        T: rusqlite::types::FromSql,
        F: FnOnce(&rusqlite::Row) -> Result<T, rusqlite::Error>,
    {
        self.with_conn(|conn| conn.query_row(sql, params, f))
    }
    /// Create or open a knowledge vault
    #[instrument(skip_all)]
    pub fn open(path: &Path, embedding_dimensions: u32) -> KnowledgeResult<Self> {
        info!("Opening knowledge vault at {:?}", path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        let vault = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path.to_path_buf(),
            embedding_dimensions,
        };

        vault.init_schema()?;

        Ok(vault)
    }

    /// Initialize database schema
    fn init_schema(&self) -> KnowledgeResult<()> {
        debug!("Initializing vault schema");

        // Documents table
        self.with_conn(|conn| conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                path TEXT UNIQUE,
                title TEXT NOT NULL,
                doc_type TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                chunk_count INTEGER NOT NULL DEFAULT 0,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                indexed_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT DEFAULT '{}'
            )
            "#,
            [],
        ))?;

        // Chunks table
        self.with_conn(|conn| conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                start_offset INTEGER NOT NULL,
                end_offset INTEGER NOT NULL,
                token_count INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}',
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )
            "#,
            [],
        ))?;

        // Embeddings table (stores raw vectors)
        self.with_conn(|conn| conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS embeddings (
                chunk_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (chunk_id) REFERENCES chunks(id) ON DELETE CASCADE
            )
            "#,
            [],
        ))?;

        // Attempt to create VSS virtual table for vector similarity search
        // This requires sqlite-vss extension to be loaded
        if let Err(e) = self.create_vss_table() {
            debug!("VSS table creation failed (extension may not be loaded): {}", e);
            debug!("Falling back to basic embedding storage without vector search");
        }

        // Indexes
        self.with_conn(|conn| conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(content_hash)",
            [],
        ))?;
        self.with_conn(|conn| conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path)",
            [],
        ))?;
        self.with_conn(|conn| conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id)",
            [],
        ))?;

        debug!("Schema initialized");
        Ok(())
    }

    /// Create VSS virtual table for vector similarity search
    fn create_vss_table(&self) -> KnowledgeResult<()> {
        // Create VSS virtual table for approximate nearest neighbor search
        // vss0 requires explicit chunk_id column for joins
        self.with_conn(|conn| conn.execute(
            &format!(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS vss_chunks USING vss0(
                    vss_chunk_id TEXT PRIMARY KEY,
                    embedding({ })
                )
                "#,
                self.embedding_dimensions
            ),
            [],
        ))?;

        debug!("VSS virtual table created with {} dimensions", self.embedding_dimensions);
        Ok(())
    }

    /// Insert a document
    #[instrument(skip(self, doc))]
    pub fn insert_document(&self, doc: &Document) -> KnowledgeResult<()> {
        self.with_conn(|conn| conn.execute(
            r#"
            INSERT INTO documents (id, path, title, doc_type, content_hash, chunk_count, size_bytes, indexed_at, updated_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content_hash = excluded.content_hash,
                chunk_count = excluded.chunk_count,
                size_bytes = excluded.size_bytes,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata
            "#,
            params![
                doc.id,
                doc.path,
                doc.title,
                doc.doc_type,
                doc.content_hash,
                doc.chunk_count,
                doc.size_bytes as i64,
                doc.indexed_at.to_rfc3339(),
                doc.updated_at.to_rfc3339(),
                serde_json::to_string(&doc.metadata).unwrap_or_default(),
            ],
        ))?;

        Ok(())
    }

    /// Get a document by ID
    pub fn get_document(&self, id: &str) -> KnowledgeResult<Option<Document>> {
        self.with_conn(|conn| {
            "SELECT id, path, title, doc_type, content_hash, chunk_count, size_bytes, indexed_at, updated_at, metadata FROM documents WHERE id = ?1"
        ))?;

                    Ok(doc = stmt.query_row(params![id], |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                doc_type: row.get(3)?,
                content_hash: row.get(4)?,
                chunk_count: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                metadata: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
            })
        }).optional()?;

        Ok(doc)
    }

    /// Check if a document exists by content hash
    pub fn has_document_hash(&self, hash: &str) -> KnowledgeResult<bool> {
        let count: i64 = self.with_conn(|conn| conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        ))?;

        Ok(count > 0)
    }

    /// Delete a document and its chunks
    pub fn delete_document(&self, id: &str) -> KnowledgeResult<()> {
        self.with_conn(|conn| conn.execute("DELETE FROM documents WHERE id = ?1", params![id]))?;
        Ok(())
    }

    /// Insert a chunk
    pub fn insert_chunk(
        &self,
        id: &str,
        document_id: &str,
        chunk_index: u32,
        content: &str,
        start_offset: u64,
        end_offset: u64,
        token_count: u32,
    ) -> KnowledgeResult<()> {
        self.with_conn(|conn| conn.execute(
            r#"
            INSERT INTO chunks (id, document_id, chunk_index, content, start_offset, end_offset, token_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                id,
                document_id,
                chunk_index,
                content,
                start_offset as i64,
                end_offset as i64,
                token_count,
            ],
        ))?;

        Ok(())
    }

    /// Get chunks for a document
    pub fn get_chunks(&self, document_id: &str) -> KnowledgeResult<Vec<ChunkRecord>> {
        self.with_conn(|conn| {
            "SELECT id, chunk_index, content, start_offset, end_offset, token_count FROM chunks WHERE document_id = ?1 ORDER BY chunk_index"
        ))?;

                    Ok(chunks = stmt.query_map(params![document_id], |row| {
            Ok(ChunkRecord {
                id: row.get(0)?,
                document_id: document_id.to_string(),
                chunk_index: row.get(1)?,
                content: row.get(2)?,
                start_offset: row.get::<_, i64>(3)? as u64,
                end_offset: row.get::<_, i64>(4)? as u64,
                token_count: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(chunks)
    }

    /// Insert an embedding
    pub fn insert_embedding(&self, chunk_id: &str, embedding: &[f32]) -> KnowledgeResult<()> {
        let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        self.with_conn(|conn| conn.execute(
            "INSERT INTO embeddings (chunk_id, embedding, created_at) VALUES (?1, ?2, ?3)",
            params![chunk_id, blob, chrono::Utc::now().to_rfc3339()],
        ))?;

        // Try to insert into VSS table for vector search
        // Format embedding as vss_debug format: "[0.1,0.2,0.3,...]"
        let embedding_str = format!(
            "[{}]",
            embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // Insert into VSS table for fast vector search
        if let Err(e) = self.with_conn(|conn| conn.execute(
            "INSERT INTO vss_chunks (vss_chunk_id, embedding) VALUES (?1, ?2)",
            params![chunk_id, embedding_str],
        )) {
            debug!("Failed to insert into VSS table (may not be available): {}", e);
        }

        Ok(())
    }

    /// Get embedding for a chunk
    pub fn get_embedding(&self, chunk_id: &str) -> KnowledgeResult<Option<Vec<f32>>> {
        let blob: Option<Vec<u8>> = self.with_conn(|conn| conn.query_row(
            "SELECT embedding FROM embeddings WHERE chunk_id = ?1",
            params![chunk_id],
            |row| row.get(0),
        ))?.optional()?;

        Ok(blob.map(|b: Vec<u8>| {
            b.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        }))
    }

    /// Search for similar chunks using vector similarity
    #[instrument(skip(self, query_embedding))]
    pub fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> KnowledgeResult<Vec<ChunkResult>> {
        debug!("Searching for top {} similar chunks", top_k);

        // Try VSS-based search first
        if let Ok(results) = self.search_vss(query_embedding, top_k) {
            return Ok(results);
        }

        // Fallback to simple cosine similarity search
        debug!("VSS search unavailable, using manual cosine similarity");
        self.search_cosine(query_embedding, top_k)
    }

    /// Search using VSS virtual table (fast approximate nearest neighbor)
    fn search_vss(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> KnowledgeResult<Vec<ChunkResult>> {
        let query_str = format!(
            "[{}]",
            query_embedding.iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let sql = format!(
            r#"
            SELECT
                c.id,
                c.document_id,
                c.content,
                d.title,
                d.path,
                vss.distance
            FROM vss_chunks
            JOIN chunks c ON vss_chunks.vss_chunk_id = c.id
            JOIN documents d ON c.document_id = d.id
            WHERE vss_chunks.embedding MATCH vss_search(?1)
            ORDER BY vss.distance
            LIMIT ?2
            "#
        );

        self.with_conn(|conn| {&sql))?;

        let results = stmt.query_map(params![query_str, top_k as i64], |row| {
            Ok(ChunkResult {
                chunk_id: row.get(0)?,
                document_id: row.get(1)?,
                content: row.get(2)?,
                document_title: row.get(3)?,
                document_path: Some(row.get(4)?),
                score: 1.0 - row.get::<_, f64>(5)? as f32, // Convert distance to similarity
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Fallback: Manual cosine similarity search
    fn search_cosine(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> KnowledgeResult<Vec<ChunkResult>> {
        // Get all embeddings with chunk info
        let sql = r#"
            SELECT
                c.id,
                c.document_id,
                c.content,
                d.title,
                d.path,
                e.embedding
            FROM embeddings e
            JOIN chunks c ON e.chunk_id = c.id
            JOIN documents d ON c.document_id = d.id
            "#;

        let mut stmt = self.with_conn(|conn| conn.prepare(sql))?;

        let mut results: Vec<ChunkResult> = stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(5)?;
            let embedding: Vec<f32> = blob.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            let score = cosine_similarity(query_embedding, &embedding);

            Ok(ChunkResult {
                chunk_id: row.get(0)?,
                document_id: row.get(1)?,
                content: row.get(2)?,
                document_title: row.get(3)?,
                document_path: row.get(4)?,
                score,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        // Sort by score descending and take top_k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        Ok(results)
    }

    /// Add a document with its content
    #[instrument(skip(self, content))]
    pub fn add_document(
        &self,
        path: &str,
        content: &str,
        doc_type: &str,
    ) -> KnowledgeResult<String> {
        use sha2::{Sha256, Digest};
        use uuid::Uuid;

        // Calculate content hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        // Check if document already exists
        if let Ok(Some(existing)) = self.get_document_by_path(path) {
            if existing.content_hash == hash {
                debug!("Document unchanged: {}", path);
                return Ok(existing.id);
            }
        }

        // Create new document
        let id = Uuid::new_v4().to_string();
        let title = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let now = chrono::Utc::now();
        let size_bytes = content.len() as u64;

        let doc = Document {
            id: id.clone(),
            path: Some(path.to_string()),
            title,
            doc_type: doc_type.to_string(),
            content_hash: hash,
            chunk_count: 0,
            size_bytes,
            indexed_at: now,
            updated_at: now,
            metadata: std::collections::HashMap::new(),
        };

        self.insert_document(&doc)?;

        Ok(id)
    }

    /// Get document by path
    fn get_document_by_path(&self, path: &str) -> KnowledgeResult<Option<Document>> {
        self.with_conn(|conn| {
            "SELECT id, path, title, doc_type, content_hash, chunk_count, size_bytes, indexed_at, updated_at, metadata FROM documents WHERE path = ?1"
        ))?;

                    Ok(doc = stmt.query_row(params![path], |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                doc_type: row.get(3)?,
                content_hash: row.get(4)?,
                chunk_count: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                metadata: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
            })
        }).optional()?;

        Ok(doc)
    }

    /// List all documents
    pub fn list_documents(&self, limit: usize) -> KnowledgeResult<Vec<Document>> {
        self.with_conn(|conn| {
            "SELECT id, path, title, doc_type, content_hash, chunk_count, size_bytes, indexed_at, updated_at, metadata FROM documents ORDER BY updated_at DESC LIMIT ?1"
        ))?;

        let docs = stmt.query_map(params![limit as i64], |row| {
            Ok(Document {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                doc_type: row.get(3)?,
                content_hash: row.get(4)?,
                chunk_count: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                indexed_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                metadata: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(docs)
    }

    /// Get vault statistics
    pub fn stats(&self) -> KnowledgeResult<VaultStats> {
        let document_count: i64 = self.with_conn(|conn| conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        ))?;

        let chunk_count: i64 = self.with_conn(|conn| conn.query_row(
            "SELECT COUNT(*) FROM chunks",
            [],
            |row| row.get(0),
        ))?;

        let embedding_count: i64 = self.with_conn(|conn| conn.query_row(
            "SELECT COUNT(*) FROM embeddings",
            [],
            |row| row.get(0),
        ))?;

        let total_size_bytes: Option<i64> = self.with_conn(|conn| conn.query_row(
            "SELECT SUM(size_bytes) FROM documents",
            [],
            |row| row.get(0),
        ))?;

        // Get database file size
        let db_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(VaultStats {
            document_count: document_count as u64,
            chunk_count: chunk_count as u64,
            embedding_count: embedding_count as u64,
            total_size_bytes: total_size_bytes.unwrap_or(0) as u64,
            database_size_bytes: db_size,
            embedding_dimensions: self.embedding_dimensions,
        })
    }
}

/// Chunk record from database
#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub id: String,
    pub document_id: String,
    pub chunk_index: u32,
    pub content: String,
    pub start_offset: u64,
    pub end_offset: u64,
    pub token_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_vault_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let vault = KnowledgeVault::open(&db_path, 384).unwrap();
        let stats = vault.stats().unwrap();

        assert_eq!(stats.document_count, 0);
        assert_eq!(stats.chunk_count, 0);
    }

    #[test]
    fn test_document_insert_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let vault = KnowledgeVault::open(&db_path, 384).unwrap();

        let doc = Document {
            id: "doc_001".to_string(),
            path: Some("/path/to/doc.md".to_string()),
            title: "Test Document".to_string(),
            doc_type: "markdown".to_string(),
            content_hash: "abc123".to_string(),
            chunk_count: 5,
            size_bytes: 1024,
            indexed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };

        vault.insert_document(&doc).unwrap();

        let retrieved = vault.get_document("doc_001").unwrap().unwrap();
        assert_eq!(retrieved.title, "Test Document");
        assert_eq!(retrieved.chunk_count, 5);
    }

    #[test]
    fn test_add_document() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let vault = KnowledgeVault::open(&db_path, 384).unwrap();

        let content = "This is a test document for the knowledge vault.";
        let doc_id = vault.add_document("/test/doc.txt", content, "text").unwrap();

        // Verify document was created
        let doc = vault.get_document(&doc_id).unwrap().unwrap();
        assert_eq!(doc.title, "doc.txt");
        assert_eq!(doc.doc_type, "text");
        assert_eq!(doc.size_bytes, content.len() as u64);

        // Adding same document again should return same ID (deduplication)
        let doc_id2 = vault.add_document("/test/doc.txt", content, "text").unwrap();
        assert_eq!(doc_id, doc_id2);
    }

    #[test]
    fn test_search_cosine_fallback() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let vault = KnowledgeVault::open(&db_path, 384).unwrap();

        // Add a document
        let doc_id = vault.add_document("/test/search.txt", "Test content for search", "text").unwrap();

        // Add a chunk
        let chunk_id = "chunk_001".to_string();
        vault.insert_chunk(
            &chunk_id,
            &doc_id,
            0,
            "Test content for search",
            0,
            20,
            5,
        ).unwrap();

        // Add an embedding
        let embedding = vec![0.1f32; 384];
        vault.insert_embedding(&chunk_id, &embedding).unwrap();

        // Search should use cosine similarity fallback
        let results = vault.search(&embedding, 5).unwrap();

        // Should find the chunk
        assert!(!results.is_empty());
        assert_eq!(results[0].chunk_id, chunk_id);
    }

    #[test]
    fn test_vault_stats() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let vault = KnowledgeVault::open(&db_path, 384).unwrap();

        let stats = vault.stats().unwrap();
        assert_eq!(stats.document_count, 0);
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.embedding_dimensions, 384);

        // Add a document
        vault.add_document("/test/stats.txt", "Content", "text").unwrap();

        let stats = vault.stats().unwrap();
        assert_eq!(stats.document_count, 1);
    }
}
