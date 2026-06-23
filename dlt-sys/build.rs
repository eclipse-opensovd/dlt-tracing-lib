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
const DLT_INCLUDE_DIR: &str = "DLT_INCLUDE_DIR";
const DLT_USER_INCLUDE_DIR: &str = "DLT_USER_INCLUDE_DIR";
const DLT_LIB_DIR: &str = "DLT_LIB_DIR";
const DLT_LIB_NAME: &str = "DLT_LIB_NAME";
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

fn env_non_empty(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn add_include_dir(build: &mut cc::Build, include: &str) {
    build.include(include).include(format!("{include}/dlt"));
}

fn add_bindgen_include(builder: bindgen::Builder, include: &str) -> bindgen::Builder {
    builder
        .clang_arg(format!("-I{include}"))
        .clang_arg(format!("-I{include}/dlt"))
}

fn main() {
    println!("cargo:rerun-if-env-changed={DLT_INCLUDE_DIR}");
    println!("cargo:rerun-if-env-changed={DLT_USER_INCLUDE_DIR}");
    println!("cargo:rerun-if-env-changed={DLT_LIB_DIR}");
    println!("cargo:rerun-if-env-changed={DLT_LIB_NAME}");

    let project_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR environment variable not set");

    let wrapper_dir = format!("{project_dir}/wrapper");

    let mut build = cc::Build::new();
    build.cpp(false).file(format!("{wrapper_dir}/{DLT_SRC}"));

    // Explicit include path overrides for unusual installations.
    if let Some(include) = env_non_empty(DLT_INCLUDE_DIR) {
        add_include_dir(&mut build, &include);
    }
    if let Some(user_include) = env_non_empty(DLT_USER_INCLUDE_DIR) {
        add_include_dir(&mut build, &user_include);
    }

    // Pass trace_load_ctrl feature to C code
    // CMake uses -DWITH_DLT_TRACE_LOAD_CTRL=ON which defines DLT_TRACE_LOAD_CTRL_ENABLE
    #[cfg(feature = "trace_load_ctrl")]
    build.define("DLT_TRACE_LOAD_CTRL_ENABLE", None);

    build.compile(DLT_WRAPPER);

    if let Some(lib_path) = env_non_empty(DLT_LIB_DIR) {
        println!("cargo:rustc-link-search=native={lib_path}");
    }
    let lib_name = env_non_empty(DLT_LIB_NAME).unwrap_or_else(|| "dlt".to_string());
    println!("cargo:rustc-link-lib=dylib={lib_name}");
    if let Ok(target_os) = std::env::var("CARGO_CFG_TARGET_OS")
        && (target_os == "linux" || target_os == "android")
    {
        println!("cargo:rustc-link-lib=dylib=dl");
    }

    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_HEADER}");
    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_SRC}");

    generate_bindings(&wrapper_dir);
}

fn generate_bindings(wrapper_dir: &str) {
    let mut builder = bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .header(format!("{wrapper_dir}/{DLT_HEADER}"));

    // Add clang args for explicit header locations.
    if let Some(include) = env_non_empty(DLT_INCLUDE_DIR) {
        builder = add_bindgen_include(builder, &include);
    }
    if let Some(user_include) = env_non_empty(DLT_USER_INCLUDE_DIR) {
        builder = add_bindgen_include(builder, &user_include);
    }
    if cfg!(feature = "trace_load_ctrl") {
        builder = builder.clang_arg("-DDLT_TRACE_LOAD_CTRL_ENABLE");
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set by Cargo");
    let target_file = std::path::PathBuf::from(out_dir).join("dlt_bindings.rs");

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
        .allowlist_function("createContext")
        .allowlist_function("freeContext")
        .allowlist_function("getContextId")
        .allowlist_function("getContextLogLevel")
        .allowlist_function("getContextTraceStatus")
        // Simple logging functions
        .allowlist_function("logDlt")
        .allowlist_function("logDltString")
        .allowlist_function("logDltUint")
        .allowlist_function("logDltInt")
        // Complex log write API
        .allowlist_function("createContextData")
        .allowlist_function("freeContextData")
        .allowlist_function("dltUserLogWriteStart")
        .allowlist_function("dltUserLogWriteFinish")
        .allowlist_function("setContextDataUserTimestamp")
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
        .unwrap_or_else(|err| panic!("Error generating bindings: {err}"))
        .write_to_file(&target_file)
        .unwrap_or_else(|err| panic!("Error writing bindings: {err}"));

    prepend_copyright(
        target_file
            .to_str()
            .expect("Invalid generated bindings path"),
    )
    .expect("Error prepending copyright header");
}

fn prepend_copyright(file_path: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(file_path)?;
    let new_content = format!("{COPYRIGHT_HEADER}{content}");
    std::fs::write(file_path, new_content)?;
    Ok(())
}
