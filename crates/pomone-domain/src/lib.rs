//! Pomone domain crate: pure types and business rules.
//!
//! This crate must remain free of I/O (no DB, no filesystem, no network).
//! All types should be deterministic and easily testable in isolation.

#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::print_stderr))]
