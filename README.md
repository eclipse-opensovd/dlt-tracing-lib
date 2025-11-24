# DLT Tracing Library

A Rust library for integrating the [tracing](https://github.com/tokio-rs/tracing) framework with [COVESA DLT (Diagnostic Log and Trace)](https://github.com/COVESA/dlt-daemon).
This project provides Rust bindings for DLT and a tracing subscriber that allows you to send structured logs and traces to DLT daemon.

## 📖 Documentation

[DLT Sys documentation](https://htmlpreview.github.io/?https://github.com/eclipse-opensovd/dlt-tracing-lib/blob/main/docs/dlt_sys/index.html)

[DLT Trace Appender documentation](https://htmlpreview.github.io/?https://github.com/eclipse-opensovd/dlt-tracing-lib/blob/main/docs/dlt_tracing_appender/index.html)


The documentation includes detailed examples, usage patterns, and API reference for all crates.
Get it by running:
```bash
 cargo doc --no-deps --all-features
```

## Overview

This workspace contains three crates:
- **`dlt-sys`** - Low-level Rust wrapper around the C libdlt library
- **`dlt-rs`** - Safe Rust API for DLT logging. This crate depends on `dlt-sys`.
- **`tracing-appender`** - Tracing subscriber/layer that integrates with the tracing framework. This crate depends on `dlt-rs`.
- **`integration-tests`** - Common test utilities for integration testing with DLT daemon

## Features

- ✅ **Type-safe Rust API** for DLT logging
- ✅ **Tracing integration** - Use standard `tracing` macros with DLT
- ✅ **Structured logging** - Field types preserved when sent to DLT
- ✅ **Span context** - Nested spans appear in log messages
- ✅ **Dynamic log levels** - Responds to DLT daemon log level changes
- ✅ **Thread-safe** - Safe for concurrent use across async tasks
- ✅ **Zero-copy** where possible for performance

## Quick Start

### Prerequisites

- Rust 1.88.0 or later

### Basic Usage

#### DLT Sys
```rust
use dlt_sys::{DltApplication, DltId, DltLogLevel};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Register application (one per process)
    let app = DltApplication::register(&DltId::new(b"MBTI")?, "Measurement & Bus Trace Interface")?;
    let ctx = app.create_context(&DltId::new(b"MEAS")?, "Measurement Context")?;

    // Simple logging
    ctx.log(DltLogLevel::Info, "Hello DLT!")?;

    // Structured logging with typed fields
    let mut writer = ctx.log_write_start(DltLogLevel::Info)?;
    writer.write_string("Temperature:")?
        .write_float32(87.5)?
        .write_string("°C")?;
    writer.finish()?;
    Ok(())
}
```

#### Dlt Tracing Appender

```rust
use dlt_tracing_appender::{DltLayer, DltId};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dlt_layer = DltLayer::new(&DltId::new(b"MBTI"), "My Beautiful Trace Ingestor")?;

    tracing_subscriber::registry().with(dlt_layer).init();

    tracing::info!("Application started");
    Ok(())
}
```

For more examples and detailed usage, see the API documentation.
The tracing and dlt-sys crates can be used simultaneously, when the application registration is done through the dlt-tracing-appender crate.

## Development

### Building

```bash
# Build all crates
cargo build

# Build with DLT load control support
cargo build --features trace_load_ctrl
```

### Running Tests

```bash
# Unit tests only (no DLT daemon required)
cargo test

# Integration tests (automatically starts DLT daemon)
cargo test -p integration-tests --features integration-tests
```

### Development Container

A devcontainer is provided with DLT daemon pre-installed. Open the project in VS Code with the Dev Containers extension.

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License
This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## References
- [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)
- [Tracing Framework](https://github.com/tokio-rs/tracing)

## Acknowledgments
This project is part of [Eclipse OpenSOVD](https://projects.eclipse.org/projects/automotive.opensovd).
See [CONTRIBUTORS](CONTRIBUTORS) for the list of contributors.
