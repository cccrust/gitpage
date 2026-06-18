# Utils — Shared Utilities

## Overview

The `utils/` module provides shared utility types used across the Gitpage backend:

- **`mod.rs`** — module re-exports (currently only the `errors` submodule)
- **`errors.rs`** — the `AppError` enum with Axum `IntoResponse` integration

## Module Structure

- `mod.rs` — declares and re-exports the `errors` module
- `errors.rs` — `AppError` enum, `IntoResponse`, `From` impls
- `README.md` — this file
