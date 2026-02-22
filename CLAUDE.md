# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Didactic real-time fraud detection pipeline for banking transactions in Rust (edition 2024). Proof-of-concept teaching Hexagonal Architecture with Dependency Inversion and TDD.

## Architecture

Cargo workspace -- modular monolith. Four pipeline components (each a lib crate) orchestrated by one binary crate:

- **Producer** -- generates transaction batches into Buffer1
- **Consumer** -- reads from Buffer1, forwards to Modelizer
- **Modelizer** -- classifies transactions (legit/fraudulent) using selectable model version
- **Logger** -- reads inferred transactions from Buffer2, persists to Storage

Buffers and Storage are trait-abstractions (hexagonal ports). All inter-component communication is async with variable batch sizes.

See `.specify/memory/constitution.md` for binding principles and `docs/fig_00.png` for pipeline diagram.

## Build Commands

```bash
cargo build --release
cargo run
cargo test
```

## Build Configuration

`.cargo/config.toml` redirects `target-dir` to `C:/Users/phili/rust_builds/Documents/Programmation/rust/13_fraud_detection_5` to avoid OneDrive sync issues. Also enables `-C target-cpu=native` for all builds.

## Speckit Workflow

`.specify/` contains templates for the speckit planning pipeline (constitution, spec, plan, tasks, checklist). Use `/speckit.*` slash commands to drive feature design and implementation.
