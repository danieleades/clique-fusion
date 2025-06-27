# Developer Guide

## Required Dependencies

- [Rust + Cargo](https://www.rust-lang.org/tools/install)
- [Cargo-Deny](https://github.com/EmbarkStudios/cargo-deny) (for CVE scanning only)

        cargo install cargo-deny
- [Cargo LLVM-Cov](https://github.com/taiki-e/cargo-llvm-cov) (for test coverage only)

        cargo install cargo-llvm-cov

## Common Tasks

### Code Check

        cargo check

### Building

        cargo build [--release]

### Formatting

        cargo fmt

### Linting

        cargo clippy

### Testing

        cargo test

### Bench Testing

        cargo bench

### Auditing

        cargo deny check
