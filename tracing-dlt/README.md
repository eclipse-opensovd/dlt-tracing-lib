# tracing-dlt
[![Crates.io](https://img.shields.io/crates/v/tracing-dlt.svg)](https://crates.io/crates/tracing-dlt)
[![Documentation](https://docs.rs/tracing-dlt/badge.svg)](https://docs.rs/tracing-dlt)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
A `tracing` subscriber/layer for sending structured logs and traces to the COVESA DLT daemon.

## Overview
`tracing-dlt` provides a [tracing](https://github.com/tokio-rs/tracing) layer that forwards logs and spans to the [COVESA DLT daemon](https://github.com/COVESA/dlt-daemon). This allows you to use the standard `tracing` macros in your Rust application while outputting to DLT.

## Features
- ✅ **Tracing integration** - Use standard `tracing::info!`, `tracing::debug!`, etc.
- ✅ **Structured logging** - Field types are preserved when sent to DLT
- ✅ **Span context** - Nested spans appear in log messages
- ✅ **Dynamic log levels** - Responds to DLT daemon log level changes
- ✅ **Thread-safe** - Safe for concurrent use across async tasks
- ✅ **Multiple contexts** - Support for different logging contexts per span

> **Note:** The `tracing-dlt` and `dlt-rs` crates can be used simultaneously in the same application, as long as application registration is done through `tracing-dlt`.

## Basic Example
```rust
use tracing_dlt::{DltLayer, DltId};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize DLT layer
    let dlt_layer = DltLayer::new(
        &DltId::new(b"MBTI")?,
        "My Beautiful Trace Ingestor"
    )?;
    // Set up tracing subscriber
    tracing_subscriber::registry()
        .with(dlt_layer)
        .init();

    // Use standard tracing macros
    tracing::info!("Application started");
    tracing::warn!(temperature = 95.5, "High temperature detected");
    // Will be logged with context ID "SPCL"
    tracing::warn!(dlt_context = "SPCL", "Log message on 'special' context id");
    Ok(())
}
```

## Features
- `trace_load_ctrl` - Enable DLT load control support (optional)
- `dlt_layer_internal_logging` - Enable debug logging for the layer itself

## License
Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.

## Contributing
This project is part of [Eclipse OpenSOVD](https://projects.eclipse.org/projects/automotive.opensovd), but can be used independently.
See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## References
- [Tracing Framework](https://github.com/tokio-rs/tracing)
- [COVESA DLT Daemon](https://github.com/COVESA/dlt-daemon)
- [DLT Protocol Specification](https://www.autosar.org/fileadmin/standards/foundation/19-11/AUTOSAR_PRS_LogAndTraceProtocol.pdf)
