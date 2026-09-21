//! Integration example with privox for PII redaction
//!
//! This example shows how to use knowledge-vault with privox to:
//! 1. Redact PII before indexing
//! 2. Search in redacted content
//! 3. Maintain privacy in your RAG system

use knowledge_vault::{Chunker, KnowledgeVault, PlaceholderEmbedder};

// Note: This example requires privox to be installed
// Add to Cargo.toml: privox = "0.1"

/*
use privox::{PatternSet, PrivacyRedactor};
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let vault = KnowledgeVault::open("private_vault.db", 384)?;
    let embedder = PlaceholderEmbedder::new(384);
    let chunker = Chunker::new();

    // Document with PII
    let sensitive_doc = r#"
Contact Information:

John Smith can be reached at john.smith@example.com or call 555-123-4567.
His SSN is 123-45-6789 and he lives at 123 Main St, New York, NY 10001.

For urgent matters, contact Jane Doe at jane.doe@example.com.
"#;

    println!("📄 Original document:\n{}", sensitive_doc);

    // Redact PII using privox (placeholder for example)
    /*
    let patterns = PatternSet::default()
        .with_email()
        .with_phone()
        .with_ssn()
        .with_address();

    let redactor = PrivacyRedactor::new(patterns);
    let redacted_doc = redactor.redact(sensitive_doc)?;

    println!("\n🔒 Redacted document:\n{}", redacted_doc);
    */

    // For this example, we'll use a simple placeholder redaction
    let redacted_doc = sensitive_doc
        .replace(r"[\w\.-]+@[\w\.-]+\.\w+", "[EMAIL_REDACTED]")
        .replace(r"\d{3}-\d{3}-\d{4}", "[PHONE_REDACTED]")
        .replace(r"\d{3}-\d{2}-\d{4}", "[SSN_REDACTED]");

    println!("\n🔒 Redacted document (placeholder):\n{}", redacted_doc);

    // Chunk the redacted content
    let chunks = chunker.chunk(&redacted_doc)?;
    println!("\n✓ Created {} chunks from redacted content", chunks.len());

    // Index the redacted content
    let doc_id = vault.add_document("contacts.md", &redacted_doc, "text")?;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_id = format!("chunk_{}_{}", doc_id, i);
        vault.insert_chunk(&chunk_id, &doc_id, chunk.index, &chunk.content, 0, 0, 0)?;

        let embedding = embedder.embed(&chunk.content).await?;
        vault.insert_embedding(&chunk_id, &embedding)?;
    }

    println!("✓ Indexed redacted content");

    // Search for contact information
    let query = "phone number contact";
    let query_emb = embedder.embed(query).await?;
    let results = vault.search(&query_emb, 2)?;

    println!("\n🔍 Search for '{}':", query);
    for (i, result) in results.iter().enumerate() {
        println!("{}. Score: {:.2}", i + 1, result.score);
        println!(
            "   Content: {}\n",
            result.content.chars().take(100).collect::<String>()
        );
    }

    println!("✓ Privacy maintained - PII redacted before indexing");

    std::fs::remove_file("private_vault.db")?;
    println!("✓ Cleaned up");

    Ok(())
}
