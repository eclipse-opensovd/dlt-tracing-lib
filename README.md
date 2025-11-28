# DLT Tracing Library

A Rust library for integrating the [tracing](https://github.com/tokio-rs/tracing) framework with [COVESA DLT (Diagnostic Log and Trace)](https://github.com/COVESA/dlt-daemon).
This project provides Rust bindings for DLT and a tracing subscriber that allows you to send structured logs and traces to DLT daemon.

## 📖 Documentation

* [tracing-dlt](https://eclipse-opensovd.github.io/dlt-tracing-lib/tracing_dlt/index.html)
* [dlt_rs](https://eclipse-opensovd.github.io/dlt-tracing-lib/dlt_rs/index.html)
* [dlr_sys](https://eclipse-opensovd.github.io/dlt-tracing-lib/dlt_sys/index.html)


## Overview

This workspace contains three publishable crates:

| Crate | Description | Documentation |
|-------|-------------|---------------|
| **[`dlt-sys`](dlt-sys/)** | Low-level FFI bindings to libdlt | [README](dlt-sys/README.md) |
| **[`dlt-rs`](dlt-rs/)** | Safe and idiomatic Rust API for DLT logging | [README](dlt-rs/README.md) |
| **[`tracing-dlt`](tracing-dlt/)** | Tracing subscriber/layer for DLT integration | [README](tracing-dlt/README.md) |

**Which crate should you use?**
- Use `tracing-dlt` for integration with the `tracing` ecosystem (recommended)
- Use `dlt-rs` for direct DLT logging with a safe API (non-tracing applications)
- Use `dlt-sys` only if building your own low-level abstraction (not recommended for most users)

See each crate's README for detailed examples and API documentation.

> **Note:** `tracing-dlt` and `dlt-rs` can be used together when application registration is done through `tracing-dlt`.

## Quick Start

### Prerequisites

- Rust 1.88.0 or later
- **libdlt** must be installed on your system


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
