/*
 * Copyright (c) 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 */

//! Low-level FFI bindings to the COVESA DLT (Diagnostic Log and Trace) C library (`libdlt`).
//!
//! # Overview
//!
//! `dlt-sys` provides unsafe Rust bindings to the
//! [COVESA DLT daemon](https://github.com/COVESA/dlt-daemon) C library.
//! This crate is intended to be used as a foundation for higher-level safe
//! Rust abstractions (see [`dlt-rs`](https://crates.io/crates/dlt-rs)).
//!
//! **Note:** This crate only implements functionality required for `dlt-rs` and does not cover
//! the entire `libdlt` API.
//!
//! # Features
//!
//! - Direct FFI bindings to `libdlt` functions
//! - Custom C wrapper for improved API ergonomics
//! - Support for all DLT log levels and message types
//! - Optional `trace_load_ctrl` feature for load control support
//!
//! # Prerequisites
//!
//! **libdlt** and its development headers must be installed on your system.
//!
//! # Usage
//!
//! This is a low-level crate with unsafe APIs. Most users should use
//! [`dlt-rs`](https://crates.io/crates/dlt-rs) instead for a safe, idiomatic Rust API.
//!
//! # Cargo Features
//!
//! - `trace_load_ctrl` - Enable DLT load control support
//! - `generate-bindings` - Regenerate bindings from C headers (development only)
//!
//! # Safety
//!
//! All functions in this crate are `unsafe` as they directly call C library functions.
//! Proper usage requires understanding of:
//! - DLT library initialization and cleanup
//! - Memory management across FFI boundaries
//! - Thread safety considerations
//!
//! For safe abstractions, use the [`dlt-rs`](https://crates.io/crates/dlt-rs) crate.
//!
//! # References
//!
//! - [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)

#[rustfmt::skip]
#[allow(clippy::all,
    dead_code,
    warnings,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
)]
mod dlt_bindings;

use std::ptr;

pub use dlt_bindings::*;

impl Default for DltContextData {
    fn default() -> Self {
        DltContextData {
            handle: ptr::null_mut(),
            buffer: ptr::null_mut(),
            size: 0,
            log_level: 0,
            trace_status: 0,
            args_num: 0,
            context_description: ptr::null_mut(),
            use_timestamp: 0,
            user_timestamp: 0,
            verbose_mode: 0,
        }
    }
}
