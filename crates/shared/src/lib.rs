// =============================================================================
// SHARED CRATE ROOT
// =============================================================================
//
// Core domain models, configuration types, route definitions, RBAC primitives,
// typed error codes, input validators, and XOR media obfuscation utilities
// shared across `services`, `api`, and `server`.
//
// ## Key Concepts
//
// - DTOs defined under `crates/shared/src/domain/` serve as the single source of
//   truth for both Axum JSON handlers and the AST TypeScript codegen (`gen-ts`).
// - Zero UI framework coupling: Svelte 5 consumes generated TypeScript bindings
//   and HTTP JSON endpoints without duplicating types.

// SECTION: PUBLIC MODULES

/// Application configuration settings and centralized route registry.
pub mod config;

/// Domain DTOs, request/response envelopes, RBAC, error codes, and validators.
pub mod domain;

/// Encoding and per-file XOR obfuscation helpers.
pub mod utils;
