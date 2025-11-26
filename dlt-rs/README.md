# dlt-rs
[![Crates.io](https://img.shields.io/crates/v/dlt-rs.svg)](https://crates.io/crates/dlt-rs)
[![Documentation](https://docs.rs/dlt-rs/badge.svg)](https://docs.rs/dlt-rs)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
Safe and idiomatic Rust wrapper for the COVESA DLT (Diagnostic Log and Trace) library.

## Overview
`dlt-rs` provides a safe, ergonomic Rust API for logging to the [COVESA DLT daemon](https://github.com/COVESA/dlt-daemon). It wraps the low-level [`dlt-sys`](https://crates.io/crates/dlt-sys) FFI bindings with a type-safe interface.

## Features
- ✅ **Type-safe API** - No unsafe code in your application
- ✅ **Structured logging** - Log typed fields (integers, floats, strings, etc.)
- ✅ **RAII-based resource management** - Automatic cleanup
- ✅ **Thread-safe** - Safe for concurrent use
- ✅ **Zero-copy** where possible for performance
- ✅ **Dynamic log levels** - Responds to DLT daemon configuration changes


## Basic Example
```rust
use dlt_rs::{DltApplication, DltId, DltLogLevel};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Register application (one per process)
    let app = DltApplication::register(
        &DltId::new(b"MBTI")?,
        "Measurement & Bus Trace Interface"
    )?;
    // Create a logging context
    let ctx = app.create_context(
        &DltId::new(b"CTX1")?,
        "Main Context"
    )?;
    // Simple text logging
    ctx.log(DltLogLevel::Info, "Hello DLT!")?;
    Ok(())
}
```

## Structured Logging
```rust
use dlt_rs::{DltLogLevel};
// Log structured data with typed fields
let mut writer = ctx.log_write_start(DltLogLevel::Info)?;
writer
    .write_string("Temperature:")?
    .write_float32(87.5)?
    .write_string("°C")?;
writer.finish()?;
```

## Tracing Integration
For integration with the `tracing` ecosystem, see the [`tracing-dlt`](https://crates.io/crates/tracing-dlt) crate.

## Features
- `trace_load_ctrl` - Enable DLT load control support (optional)

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

## Contributing
This project is part of [Eclipse OpenSOVD](https://projects.eclipse.org/projects/automotive.opensovd), but can be used independently.
See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## References
- [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)
- [DLT Protocol Specification](https://www.autosar.org/fileadmin/standards/foundation/19-11/AUTOSAR_PRS_LogAndTraceProtocol.pdf)
