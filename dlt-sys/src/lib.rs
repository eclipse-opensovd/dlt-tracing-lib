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
