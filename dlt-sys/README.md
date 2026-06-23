# dlt-sys
[![Crates.io](https://img.shields.io/crates/v/dlt-sys.svg)](https://crates.io/crates/dlt-sys)
[![Documentation](https://docs.rs/dlt-sys/badge.svg)](https://docs.rs/dlt-sys)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
Low-level FFI bindings to the COVESA DLT (Diagnostic Log and Trace) C library (`libdlt`).

## Overview
`dlt-sys` provides unsafe Rust bindings to the [COVESA DLT daemon](https://github.com/COVESA/dlt-daemon) C library.
This crate is intended to be used as a foundation for higher-level safe Rust abstractions (see [`dlt-rs`](https://crates.io/crates/dlt-rs)).
Please note that this is only implements functionality required for dlt-rs and does not cover the entire libdlt API.

## Features
- Direct FFI bindings to `libdlt` functions
- Custom C wrapper for improved API ergonomics
- Support for all DLT log levels and message types
- Optional `trace_load_ctrl` feature for load control support

## Prerequisites
- **libdlt** and its development headers must be installed on your system.  
  On Debian-based systems, this can be installed with: `apt install libdlt-dev`
- **bindgen prerequisites** must be available at build time (`clang` and `libclang`).
  `dlt-sys` generates Rust bindings on the fly during `cargo build`, using the installed DLT headers.

## Usage
This is a low-level crate with unsafe APIs. Most users should use [`dlt-rs`](https://crates.io/crates/dlt-rs) instead for a safe, idiomatic Rust API.

## Features
- `trace_load_ctrl` - Enable DLT load control support (may be required in some environments, depending on the DLT build time daemon configuration)
- `generate-bindings` - Regenerate bindings from C headers (development only)

## Compatibility Across DLT Releases
- `dlt-sys` is designed to work with multiple `libdlt` releases.
- Bindings are generated at build time from the locally installed DLT headers, so the Rust FFI shape matches the installed release.
- `DltContextData` uses zero-initialization in Rust to remain source-compatible when `libdlt` adds fields.
- Optional C APIs are resolved dynamically in the wrapper at runtime.
  If `dlt_register_log_level_changed_callback` is unavailable in an older `libdlt`, callback registration returns a DLT error instead of failing at link time.
- In CI, the repository setup action supports selecting a DLT daemon ref (`dlt-ref`) so workflows can validate multiple releases.

### Selecting The Linked DLT Release
When consuming this crate from crates.io, select the installed DLT release by pointing include and library paths to that installation.

- `DLT_INCLUDE_DIR`: include directory containing `dlt/` headers
- `DLT_LIB_DIR`: library directory containing `libdlt`
- `DLT_USER_INCLUDE_DIR`: optional additional include directory
- `DLT_LIB_NAME`: optional library name override (default: `dlt`)

Example:

```bash
DLT_INCLUDE_DIR=/opt/homebrew/include \
DLT_LIB_DIR=/opt/homebrew/lib \
cargo build
```

## Safety
All functions in this crate are `unsafe` as they directly call C library functions. Proper usage requires understanding of:
- DLT library initialization and cleanup
- Memory management across FFI boundaries
- Thread safety considerations
For safe abstractions, use the [`dlt-rs`](https://crates.io/crates/dlt-rs) crate.

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
## Contributing
This project is part of [Eclipse OpenSOVD](https://projects.eclipse.org/projects/automotive.opensovd), but can be used independently.
See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## References
- [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)
- [DLT Protocol Specification](https://www.autosar.org/fileadmin/standards/foundation/19-11/AUTOSAR_PRS_LogAndTraceProtocol.pdf)
