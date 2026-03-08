//! Application service boundaries consumed by presentation layers.
//!
//! Ownership:
//! - UI calls only service interfaces.
//! - Service implementations orchestrate application workflows.
//! - Lower-level modules remain internal implementation details.

pub mod workspace_service;
