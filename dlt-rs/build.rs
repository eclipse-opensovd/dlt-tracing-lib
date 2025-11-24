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

fn main() {
    #[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
    {
        println!(
            "cargo:warning=write_float64 can cause an illegal instruction error on ARM32 due to a \
             known issue in libdlt"
        );
    }
}
