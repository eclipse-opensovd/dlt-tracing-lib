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

const DLT_WRAPPER: &str = "dlt-wrapper";
const DLT_HEADER: &str = "dlt-wrapper.h";
const DLT_SRC: &str = "dlt-wrapper.c";
#[cfg(feature = "generate-bindings")]
const COPYRIGHT_HEADER: &str = r"/*
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

";

// necessary to ensure that bindings are generated with trace_load_ctrl enabled
// otherwise we cannot enable the feature with the generated bindings as some types will be missing
#[cfg(all(feature = "generate-bindings", not(feature = "trace_load_ctrl")))]
compile_error!("Feature 'generate-bindings' requires 'trace_load_ctrl' to be enabled");

fn main() {
    let project_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR environment variable not set");

    let wrapper_dir = format!("{project_dir}/wrapper");

    let mut build = cc::Build::new();
    build.cpp(false).file(format!("{wrapper_dir}/{DLT_SRC}"));

    // Add system DLT include paths
    if let Ok(include) = std::env::var("DLT_INCLUDE_DIR") {
        build.include(&include).include(format!("{include}/dlt"));
    }
    if let Ok(user_include) = std::env::var("DLT_USER_INCLUDE_DIR") {
        build
            .include(&user_include)
            .include(format!("{user_include}/dlt"));
    }

    // Pass trace_load_ctrl feature to C code
    // CMake uses -DWITH_DLT_TRACE_LOAD_CTRL=ON which defines DLT_TRACE_LOAD_CTRL_ENABLE
    #[cfg(feature = "trace_load_ctrl")]
    build.define("DLT_TRACE_LOAD_CTRL_ENABLE", None);

    build.compile(DLT_WRAPPER);

    if let Ok(lib_path) = std::env::var("DLT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={lib_path}");
    }
    println!("cargo:rustc-link-lib=dylib=dlt");

    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_HEADER}");
    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_SRC}");

    #[cfg(feature = "generate-bindings")]
    generate_bindings(&project_dir, &wrapper_dir);
}

#[cfg(feature = "generate-bindings")]
fn generate_bindings(project_dir: &str, wrapper_dir: &str) {
    let mut builder = bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .header(format!("{}/{}", wrapper_dir, DLT_HEADER));

    // Add clang args for system DLT headers
    if let Ok(include) = std::env::var("DLT_INCLUDE_DIR") {
        builder = builder.clang_arg(format!("-I{}", include));
    }
    if let Ok(user_include) = std::env::var("DLT_USER_INCLUDE_DIR") {
        builder = builder.clang_arg(format!("-I{}", user_include));
    }
    if cfg!(feature = "trace_load_ctrl") {
        builder = builder.clang_arg("-DDLT_TRACE_LOAD_CTRL_ENABLE");
    }

    let target_file = std::path::PathBuf::from(project_dir).join("src/dlt_bindings.rs");

    builder
        // Types
        .allowlist_type("DltContext")
        .allowlist_type("DltContextData")
        .allowlist_type("DltLogLevelType")
        .allowlist_type("DltTimestampType")
        .allowlist_type("DltReturnValue")
        .allowlist_type("DltTraceStatusType")
        // Constants
        .allowlist_var("DLT_ID_SIZE")
        .allowlist_var("DLT_LOG_.*")
        .allowlist_var("DLT_RETURN_.*")
        // Application management functions
        .allowlist_function("registerApplication")
        .allowlist_function("unregisterApplicationFlushBufferedLogs")
        .allowlist_function("dltFree")
        // Context management functions
        .allowlist_function("registerContext")
        .allowlist_function("unregisterContext")
        // Simple logging functions
        .allowlist_function("logDlt")
        .allowlist_function("logDltString")
        .allowlist_function("logDltUint")
        .allowlist_function("logDltInt")
        // Complex log write API
        .allowlist_function("dltUserLogWriteStart")
        .allowlist_function("dltUserLogWriteFinish")
        .allowlist_function("dltUserLogWriteString")
        .allowlist_function("dltUserLogWriteUint")
        .allowlist_function("dltUserLogWriteInt")
        .allowlist_function("dltUserLogWriteUint64")
        .allowlist_function("dltUserLogWriteInt64")
        .allowlist_function("dltUserLogWriteFloat32")
        .allowlist_function("dltUserLogWriteFloat64")
        .allowlist_function("dltUserLogWriteBool")
        // Callback registration
        .allowlist_function("registerLogLevelChangedCallback")
        .generate()
        .unwrap_or_else(|err| panic!("Error generating bindings: {}", err))
        .write_to_file(&target_file)
        .unwrap_or_else(|err| panic!("Error writing bindings: {}", err));

    prepend_copyright(target_file.to_str().unwrap()).unwrap();
}

#[cfg(feature = "generate-bindings")]
fn prepend_copyright(file_path: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(file_path)?;
    let new_content = format!("{COPYRIGHT_HEADER}{content}");
    std::fs::write(file_path, new_content)?;
    Ok(())
}
