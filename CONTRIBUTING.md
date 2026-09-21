# Contributing to Knowledge Vault

Thank you for your interest in contributing to Knowledge Vault! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:
- Be respectful and inclusive
- Provide constructive feedback
- Focus on what is best for the community
- Show empathy towards other community members

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates. When creating a bug report, include:

- **Clear title and description**
- **Steps to reproduce** the issue
- **Expected behavior** vs **actual behavior**
- **Environment details** (OS, Rust version, etc.)
- **Relevant logs** or error messages
- **Minimal reproducible example** if possible

### Suggesting Enhancements

Enhancement suggestions are welcome! Please:

- **Use a clear title** for the issue
- **Provide a detailed description** of the suggested enhancement
- **Explain the use case** and why it would be useful
- **Consider including** mockups or examples

### Pull Requests

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** with clear, descriptive commit messages
3. **Add tests** for new functionality or bug fixes
4. **Ensure all tests pass**: `cargo test`
5. **Check formatting**: `cargo fmt`
6. **Run clippy**: `cargo clippy --all-targets`
7. **Update documentation** as needed
8. **Submit your pull request** with a clear description of changes

#### Development Setup

```bash
# Clone your fork
git clone https://github.com/your-username/knowledge-vault.git
cd knowledge-vault

# Install dependencies
cargo build

# Run tests
cargo test

# Run examples
cargo run --example basic_indexing
```

#### Coding Standards

- **Follow Rust style guidelines**: Use `cargo fmt` for formatting
- **Zero warnings**: Ensure `cargo clippy --all-targets` passes with no warnings
- **Documentation**: Add doc comments to public APIs
- **Tests**: Write unit tests for new functionality
- **Examples**: Provide examples for new features

### Code Review Process

- All submissions require review before merging
- Address review feedback promptly
- Keep discussions focused and constructive
- Request review from maintainers when ready

## Development Guidelines

### Architecture

Knowledge Vault follows a modular architecture:

- **Vault**: SQLite storage and vector search
- **Chunker**: Document chunking strategies
- **Embeddings**: Pluggable embedding providers
- **Indexer**: Document ingestion pipeline
- **Watcher**: File change monitoring
- **Search**: Semantic and keyword search

When adding features, consider:
- **Modularity**: Keep components independent
- **Extensibility**: Use traits for pluggable behavior
- **Performance**: Profile and optimize hot paths
- **Testing**: Ensure comprehensive coverage

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_vault_creation

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### Documentation

- **Public APIs**: Must have doc comments
- **Examples**: Include usage examples in docs
- **README**: Update for user-facing changes
- **CHANGELOG**: Add entries for significant changes

### Commit Messages

Follow conventional commit format:

```
feat: add support for custom chunking strategies
fix: resolve race condition in indexer
docs: update README with new examples
test: add tests for vector search
refactor: simplify embedder trait
perf: optimize cosine similarity calculation
```

## Project Structure

```
knowledge-vault/
├── src/
│   ├── lib.rs          # Public API and re-exports
│   ├── vault.rs        # SQLite storage and search
│   ├── chunker.rs      # Document chunking
│   ├── embeddings.rs   # Embedding providers
│   ├── indexer.rs      # Document indexing
│   ├── watcher.rs      # File watching
│   └── search.rs       # Search functionality
├── examples/           # Usage examples
├── benches/            # Performance benchmarks
├── tests/              # Integration tests
└── docs/               # Additional documentation
```

## Getting Help

- **Issues**: Use GitHub Issues for bugs and feature requests
- **Discussions**: Use GitHub Discussions for questions and ideas
- **Documentation**: Check the [docs.rs] documentation
- **Examples**: Review the [examples] directory

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.

## Recognition

Contributors will be recognized in the CONTRIBUTORS.md file. Thank you for your contributions!

---

**We appreciate your contributions to making Knowledge Vault better!** 🎉
