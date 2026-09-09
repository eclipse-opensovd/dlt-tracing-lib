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

use std::path::{Path, PathBuf};

const DLT_WRAPPER: &str = "dlt-wrapper";
const DLT_HEADER: &str = "dlt-wrapper.h";
const DLT_SRC: &str = "dlt-wrapper.c";
const DLT_INCLUDE_DIR: &str = "DLT_INCLUDE_DIR";
const DLT_USER_INCLUDE_DIR: &str = "DLT_USER_INCLUDE_DIR";
const DLT_LIB_DIR: &str = "DLT_LIB_DIR";
const DLT_LIB_NAME: &str = "DLT_LIB_NAME";
const DLT_NO_PKG_CONFIG: &str = "DLT_NO_PKG_CONFIG";

/// Name of the pkg-config module installed by dlt-daemon (`WITH_DLT_PKGCONFIG=ON`).
const DLT_PKG_CONFIG_NAME: &str = "automotive-dlt";

/// Default library name, overridable through [`DLT_LIB_NAME`].
const DLT_DEFAULT_LIB_NAME: &str = "dlt";

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

/// Where libdlt was found, regardless of how it was discovered.
#[derive(Default)]
struct DltLocation {
    /// Include directories, ready to be passed to the compiler.
    include_dirs: Vec<PathBuf>,
    /// Library search directories.
    link_dirs: Vec<PathBuf>,
}

fn env_non_empty(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Adds an include directory the way libdlt headers may be reached.
///
/// Depending on the installation, `dlt/dlt.h` resolves either below the prefix or
/// below a `dlt` subdirectory, so both are offered to the compiler.
fn push_include_dir(dirs: &mut Vec<PathBuf>, include: &str) {
    dirs.push(PathBuf::from(include));
    dirs.push(Path::new(include).join("dlt"));
}

/// Explicit include and library paths, for unusual installations.
///
/// Setting any of these suppresses the pkg-config probe, so the configured paths
/// are the ones used.
fn location_from_env() -> Option<DltLocation> {
    let include = env_non_empty(DLT_INCLUDE_DIR);
    let user_include = env_non_empty(DLT_USER_INCLUDE_DIR);
    let lib_dir = env_non_empty(DLT_LIB_DIR);

    if include.is_none() && user_include.is_none() && lib_dir.is_none() {
        return None;
    }

    let mut location = DltLocation::default();
    for dir in [include, user_include].into_iter().flatten() {
        push_include_dir(&mut location.include_dirs, &dir);
    }
    if let Some(lib_dir) = lib_dir {
        location.link_dirs.push(PathBuf::from(lib_dir));
    }
    Some(location)
}

/// Warns about pkg-config failures the fallback cannot recover from. A plain probe
/// failure stays quiet: DLT may still sit on the default search path.
fn warn_pkg_config_failure(err: &pkg_config::Error) {
    if matches!(err, pkg_config::Error::ProbeFailure { .. }) {
        return;
    }
    let detail = err.to_string().replace('\n', " ");
    println!("cargo:warning=pkg-config could not be consulted for {DLT_PKG_CONFIG_NAME}: {detail}");
}

/// Asks pkg-config where libdlt is.
///
/// Link lines are emitted by this script rather than by pkg-config, so that the
/// flags stay under the control of [`emit_link_flags`].
fn location_from_pkg_config() -> Option<DltLocation> {
    let library = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe(DLT_PKG_CONFIG_NAME)
        .inspect_err(warn_pkg_config_failure)
        .ok()?;

    Some(DltLocation {
        include_dirs: library.include_paths,
        link_dirs: library.link_paths,
    })
}

/// Locates libdlt: explicit environment first, then pkg-config, then compiler defaults.
fn locate() -> DltLocation {
    if let Some(location) = location_from_env() {
        return location;
    }
    if env_non_empty(DLT_NO_PKG_CONFIG).is_none()
        && let Some(location) = location_from_pkg_config()
    {
        return location;
    }
    DltLocation::default()
}

fn emit_link_flags(location: &DltLocation, lib_name: &str, target_os: &str) {
    for dir in &location.link_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    println!("cargo:rustc-link-lib=dylib={lib_name}");

    if target_os == "linux" || target_os == "android" {
        // The wrapper resolves optional libdlt APIs with dlsym.
        println!("cargo:rustc-link-lib=dylib=dl");
    }
}

fn main() {
    for var in [
        DLT_INCLUDE_DIR,
        DLT_USER_INCLUDE_DIR,
        DLT_LIB_DIR,
        DLT_LIB_NAME,
        DLT_NO_PKG_CONFIG,
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let project_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR environment variable not set");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is not set by Cargo"));
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let wrapper_dir = format!("{project_dir}/wrapper");

    let lib_name = env_non_empty(DLT_LIB_NAME).unwrap_or_else(|| DLT_DEFAULT_LIB_NAME.to_string());
    let location = locate();

    let mut build = cc::Build::new();
    build.cpp(false).file(format!("{wrapper_dir}/{DLT_SRC}"));
    for dir in &location.include_dirs {
        build.include(dir);
    }

    // Pass trace_load_ctrl feature to C code
    // CMake uses -DWITH_DLT_TRACE_LOAD_CTRL=ON which defines DLT_TRACE_LOAD_CTRL_ENABLE
    #[cfg(feature = "trace_load_ctrl")]
    build.define("DLT_TRACE_LOAD_CTRL_ENABLE", None);

    build.compile(DLT_WRAPPER);

    emit_link_flags(&location, &lib_name, &target_os);

    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_HEADER}");
    println!("cargo:rerun-if-changed={wrapper_dir}/{DLT_SRC}");

    generate_bindings(&wrapper_dir, &location, &out_dir);
}

fn generate_bindings(wrapper_dir: &str, location: &DltLocation, out_dir: &Path) {
    let mut builder = bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .header(format!("{wrapper_dir}/{DLT_HEADER}"));

    for dir in &location.include_dirs {
        builder = builder.clang_arg(format!("-I{}", dir.display()));
    }
    if cfg!(feature = "trace_load_ctrl") {
        builder = builder.clang_arg("-DDLT_TRACE_LOAD_CTRL_ENABLE");
    }

    let target_file = out_dir.join("dlt_bindings.rs");

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
