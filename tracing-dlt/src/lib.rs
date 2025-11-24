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

//! Tracing integration for COVESA DLT (Diagnostic Log and Trace)
//!
//! [`tracing`](https://docs.rs/tracing) subscriber layer that forwards tracing events to the
//! DLT daemon, enabling standard `tracing` macros (`info!`, `debug!`, `error!`, etc.) to
//! send logs to DLT for centralized diagnostics.
//!
//! # Quick Start
//!
//! ```no_run
//! use tracing_dlt::{DltLayer, DltId};
//! use tracing::{info, span, Level};
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Register DLT layer
//! let app_id = DltId::new(b"MBTI")?;
//! tracing_subscriber::registry()
//!     .with(DltLayer::new(&app_id, "My Beautiful Trace Ingestor")?)
//!     .init();
//!
//! // Basic logging (uses default "DFLT" context)
//! info!("Application started");
//! // DLT Output: MBTI DFLT log info V 1 [lib: Application started]
//! //             ^^^^ ^^^^ ^^^ ^^^^ ^ ^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//! //             |    |    |   |    | |  Message (crate + msg)
//! //             |    |    |   |    | Number of arguments
//! //             |    |    |   |    Verbose flag
//! //             |    |    |   Log level
//! //             |    |    Log type
//! //             |    Context ID (default)
//! //             Application ID
//!
//! // Custom DLT context per span
//! let span = span!(Level::INFO, "network", dlt_context = "NET");
//! let _guard = span.enter();
//! info!(bytes = 1024, "Data received");
//! // DLT Output: MBTI NET log info V 2 [network: lib: Data received bytes = 1024]
//! //                  ^^^               ^^^^^^^^       ^^^^^^^^^^^^        ^^^^
//! //             |    Custom context    Span name      Message             Structured field
//! # Ok(())
//! # }
//! ```
//!
//! # Core Features
//!
//! - **Per-span contexts** - Use `dlt_context` field to route logs to specific DLT contexts
//! - **Structured logging** - Span fields automatically included in messages with native types
//! - **Layer composition** - Combine with other tracing layers (fmt, file, etc.)
//! - **Thread-safe** - Full `Send + Sync` support
//!
//! # DLT Context Management
//!
//! Events outside spans use the default "DFLT" context. Spans can specify their own
//! context with the `dlt_context` field (auto-creates and caches contexts):
//!
//! ```no_run
//! # use tracing_dlt::{DltLayer, DltId};
//! # use tracing::{info, span, Level};
//! # use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let app_id = DltId::new(b"ADAS")?;
//! # tracing_subscriber::registry().with(DltLayer::new(&app_id, "ADAS")?)  .init();
//! // Nested contexts
//! let outer = span!(Level::INFO, "can_bus", dlt_context = "CAN");
//! let _g1 = outer.enter();
//! info!(msg_id = 0x123, "CAN frame received");
//!
//! let inner = span!(Level::DEBUG, "decode", dlt_context = "CTRL");
//! let _g2 = inner.enter();
//! info!("Decoded steering command");
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`dlt_sys`] - Low-level Rust bindings for DLT
//! - [`tracing`](https://docs.rs/tracing) - Application-level tracing framework
//! - [COVESA DLT](https://github.com/COVESA/dlt-daemon) - The DLT daemon

use std::{
    collections::HashMap,
    fmt,
    fmt::Write,
    sync::{Arc, RwLock},
};

use dlt_rs::{DltContextHandle};
// Re-export types for users of this library
pub use dlt_rs::{DltError, DltId, DltSysError, DltLogLevel, DltApplication};
use indexmap::IndexMap;
use tracing_core::{Event, Subscriber, span};
use tracing_subscriber::{Layer, filter::LevelFilter, layer::Context, registry::LookupSpan};

/// Field name for custom DLT context ID in spans
const DLT_CONTEXT_FIELD: &str = "dlt_context";

/// COVESA DLT layer for tracing
///
/// Integrates `tracing` with DLT (Diagnostic Log and Trace). Spans can specify their
/// own DLT context via the `dlt_context` field; events outside spans use "DFLT" context.
///
/// See the [crate-level documentation](index.html) for usage examples.
pub struct DltLayer {
    pub app: Arc<DltApplication>,
    /// Default context for events outside of spans
    default_context: Arc<DltContextHandle>,
    /// Cache of DLT contexts by span name
    context_cache: Arc<RwLock<HashMap<String, Arc<DltContextHandle>>>>,
}
impl DltLayer {
    /// Create a new DLT layer
    ///
    /// Registers the application with DLT and creates a default "DFLT" context.
    /// Span contexts are created on-demand using the `dlt_context` field.
    ///
    /// # Errors
    /// Returns error if DLT registration fails or strings contain null bytes.
    ///
    /// # Panics
    /// If called outside a Tokio runtime context.
    pub fn new(app_id: &DltId, app_description: &str) -> Result<Self, DltError> {
        // Register application with DLT
        let app = Arc::new(DltApplication::register(app_id, app_description)?);
        // Create default context for events outside spans
        let default_context =
            Arc::new(app.create_context(&DltId::new(b"DFLT")?, "Default context")?);

        Self::register_context_level_changed(&default_context)?;

        Ok(DltLayer {
            app,
            default_context,
            context_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn register_context_level_changed(context: &DltContextHandle) -> Result<(), DltError> {
        let mut receiver = context.register_log_level_changed_listener()?;
        // Spawn a background task to handle log level updates for all contexts
        tokio::spawn(async move {
            while let Ok(_event) = receiver.recv().await {
                // Log level is already updated internally in the DltContextHandle
                // We just need to rebuild the callsite interest
                // cache so that the new level takes effect
                tracing_core::callsite::rebuild_interest_cache();
            }
        });
        Ok(())
    }

    /// Get or create a DLT context for a span
    ///
    /// Checks for a `dlt_context` field in the span. If present, uses it as the context ID.
    /// Otherwise, returns the default context.
    /// Caches contexts to avoid recreating them for the same context ID.
    fn get_or_create_context_for_span<S>(
        &self,
        span: &tracing_subscriber::registry::SpanRef<'_, S>,
    ) -> Result<Arc<DltContextHandle>, DltError>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let span_name = span.name();

        // Check if span has a custom "dlt_context" field
        let dlt_context_id = {
            let extensions = span.extensions();
            extensions
                .get::<IndexMap<String, FieldValue>>()
                .and_then(|fields| {
                    fields.get(DLT_CONTEXT_FIELD).and_then(|value| match value {
                        FieldValue::Str(s) => Some(s.clone()),
                        _ => None,
                    })
                })
        };

        // If no custom dlt_context field, use default context
        let Some(custom_id) = dlt_context_id else {
            return Ok(Arc::clone(&self.default_context));
        };

        // Check cache for custom context
        {
            let cache = self.context_cache.read().map_err(|_| DltError::BadLock)?;
            if let Some(context) = cache.get(&custom_id) {
                return Ok(Arc::clone(context));
            }
        }

        // Create new context with custom ID
        let ctx_id = Self::span_name_to_dlt_id(&custom_id)?;
        let context = Arc::new(self.app.create_context(&ctx_id, span_name)?);

        let mut cache = self.context_cache.write().map_err(|_| DltError::BadLock)?;
        cache.insert(custom_id, Arc::clone(&context));
        Self::register_context_level_changed(&context)?;

        Ok(context)
    }

    /// Convert a span name to a valid DLT context ID
    ///
    /// Takes the first 1-4 bytes of the span name, uppercase.
    /// If the name is longer, it's truncated. If shorter, it's used as-is.
    fn span_name_to_dlt_id(name: &str) -> Result<DltId, DltError> {
        if name.is_empty() {
            return Err(DltError::InvalidInput);
        }
        let bytes = name.as_bytes();
        let len = bytes.len().clamp(1, dlt_rs::DLT_ID_SIZE_USIZE);

        let get = |i| bytes.get(i).copied().ok_or(DltError::InvalidInput);

        match len {
            1 => DltId::new(&[get(0)?]),
            2 => DltId::new(&[get(0)?, get(1)?]),
            3 => DltId::new(&[get(0)?, get(1)?, get(2)?]),
            _ => DltId::new(&[get(0)?, get(1)?, get(2)?, get(3)?]),
        }
    }

    #[cfg(feature = "dlt_layer_internal_logging")]
    fn log_dlt_error(metadata: &tracing_core::Metadata, level: tracing::Level, e: DltSysError) {
        eprintln!("DLT error occurred: {e:?}");
        tracing::warn!(
            target: "dlt_layer_internal",
            error = ?e,
            event_target = metadata.target(),
            event_level = ?level,
            "DLT error occurred"
        );
    }

    #[cfg(not(feature = "dlt_layer_internal_logging"))]
    fn log_dlt_error(_metadata: &tracing_core::Metadata, _level: tracing::Level, _e: DltSysError) {
        // Silent if internal logging is disabled
    }

    fn max_dlt_level(&self) -> DltLogLevel {
        if let Ok(cache) = self.context_cache.read() {
            cache
                .values()
                .map(|ctx| ctx.log_level())
                .chain(std::iter::once(self.default_context.log_level()))
                .max_by_key(|&level| level as i32)
                .unwrap_or(DltLogLevel::Default)
        } else {
            DltLogLevel::Default
        }
    }
}

impl<S> Layer<S> for DltLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        // Prevent our own internal error messages from being logged to DLT
        metadata.target() != "dlt_layer_internal"
    }

    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        // Store span fields for later context building
        if let Some(span) = ctx.span(id) {
            let mut visitor = FieldVisitor::new();
            attrs.record(&mut visitor);

            let mut extensions = span.extensions_mut();
            extensions.insert(visitor.fields);
        }
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        // Update span fields
        if let Some(span) = ctx.span(span) {
            let mut visitor = FieldVisitor::new();
            values.record(&mut visitor);

            let mut extensions = span.extensions_mut();
            if let Some(fields) = extensions.get_mut::<IndexMap<String, FieldValue>>() {
                fields.extend(visitor.fields);
            } else {
                extensions.insert(visitor.fields);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level();

        // Determine which DLT context to use based on the current span
        let dlt_context = ctx
            .event_scope(event)
            .and_then(|scope| scope.from_root().last())
            .map_or(Arc::clone(&self.default_context), |span| {
                self.get_or_create_context_for_span(&span)
                    .unwrap_or_else(|_| Arc::clone(&self.default_context))
            });

        let dlt_level = map_level_to_dlt(*level);
        let context_log_level = dlt_context.log_level();
        if (dlt_level as i32) > (context_log_level as i32) {
            return; // Skip logging if level is above DLT threshold for this context
        }

        // Start building the DLT message
        let mut log_writer = match dlt_context.log_write_start(dlt_level) {
            Ok(log_writer) => log_writer,
            Err(e) => {
                Self::log_dlt_error(metadata, *level, e);
                return;
            }
        };

        if let Some(scope) = ctx.event_scope(event) {
            let mut span_context = String::new();
            for span in scope.from_root() {
                if !span_context.is_empty() {
                    span_context.push(':');
                }
                span_context.push_str(span.name());

                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<IndexMap<String, FieldValue>>() {
                    // Filter out dlt_context field from display
                    let display_fields: Vec<_> = fields
                        .iter()
                        .filter(|(name, _)| *name != DLT_CONTEXT_FIELD)
                        .collect();

                    if !display_fields.is_empty() {
                        span_context.push('{');
                        for (i, (k, v)) in display_fields.iter().enumerate() {
                            if i > 0 {
                                span_context.push_str(", ");
                            }
                            let _ = write!(span_context, "{k}={v}");
                        }
                        span_context.push('}');
                    }
                }
            }

            if !span_context.is_empty() {
                span_context.push(':');
                let _ = log_writer.write_string(&span_context);
            }
        }

        // Add event fields with native types
        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        // Extract the message field if present, so we can
        // only take the value without the "message=" prefix
        // this reduces the clutter in dlt and gives more space for relevant info
        let mut fields = visitor.fields;
        let message_field = fields.shift_remove("message");
        let target = metadata.target();
        if let Some(msg) = message_field {
            let formatted = if target.is_empty() {
                msg.to_string()
            } else {
                format!("{target}: {msg}")
            };
            let _ = log_writer.write_string(&formatted);
        } else if !target.is_empty() {
            let _ = log_writer.write_string(target);
        }

        // Write all other fields normally, the dlt_context field is already filtered out
        if let Err(e) = write_fields(&mut log_writer, fields) {
            Self::log_dlt_error(metadata, *level, e);
            return;
        }

        // Finish and send the log message
        if let Err(e) = log_writer.finish() {
            Self::log_dlt_error(metadata, *level, e);
        }
    }
}

// Implement Filter trait to provide dynamic max level hint based on DLT configuration
impl<S> tracing_subscriber::layer::Filter<S> for DltLayer {
    /// Determines if a span or event with the given metadata is enabled.
    ///
    /// This implementation checks if the event/span level is within the current
    /// DLT maximum log level across all contexts. It also filters out internal
    /// DLT layer errors to prevent recursion.
    ///
    /// Since we don't know which context will be used at this point, we use the
    /// most permissive level across all contexts.
    fn enabled(&self, meta: &tracing_core::Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        // Prevent our own internal error messages from being logged to DLT
        if meta.target() == "dlt_layer_internal" {
            return false;
        }

        // Check if this log level is enabled by any DLT context
        let dlt_level = map_level_to_dlt(*meta.level());

        // Find the most permissive (highest) log level across all contexts
        let max_level = self.max_dlt_level();

        // Compare log levels - enable if event level is less than or equal to max allowed
        (dlt_level as i32) <= (max_level as i32)
    }

    /// Returns the current maximum log level from DLT.
    ///
    /// This hint allows the tracing infrastructure to skip callsites that are
    /// more verbose than the current DLT log level, improving performance.
    ///
    /// When the DLT log level changes (via DLT daemon or configuration), the
    /// background task calls `rebuild_interest_cache()` to ensure this new hint
    /// takes effect immediately.
    ///
    /// This returns the most permissive (verbose) level across all contexts, since
    /// we can't know which context will be used at callsite registration time.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        let max_level = self.max_dlt_level();

        Some(map_dlt_to_level_filter(max_level))
    }
}

/// Map tracing log levels to DLT log levels
fn map_level_to_dlt(level: tracing::Level) -> DltLogLevel {
    match level {
        tracing::Level::ERROR => DltLogLevel::Error,
        tracing::Level::WARN => DltLogLevel::Warn,
        tracing::Level::INFO => DltLogLevel::Info,
        tracing::Level::DEBUG => DltLogLevel::Debug,
        tracing::Level::TRACE => DltLogLevel::Verbose,
    }
}

/// Map DLT log level to tracing `LevelFilter`
fn map_dlt_to_level_filter(dlt_level: DltLogLevel) -> LevelFilter {
    match dlt_level {
        DltLogLevel::Off | DltLogLevel::Default => LevelFilter::OFF,
        DltLogLevel::Fatal | DltLogLevel::Error => LevelFilter::ERROR,
        DltLogLevel::Warn => LevelFilter::WARN,
        DltLogLevel::Info => LevelFilter::INFO,
        DltLogLevel::Debug => LevelFilter::DEBUG,
        DltLogLevel::Verbose => LevelFilter::TRACE,
    }
}

/// Helper function to write fields to DLT with proper error propagation
fn write_fields(
    log_writer: &mut dlt_rs::DltLogWriter,
    fields: IndexMap<String, FieldValue>,
) -> Result<(), DltSysError> {
    for (field_name, field_value) in fields {
        // Write field name
        log_writer.write_string(&field_name)?;
        log_writer.write_string("=")?;

        // Write field value with native type
        match field_value {
            FieldValue::I64(v) => {
                log_writer.write_int64(v)?;
            }
            FieldValue::U64(v) => {
                log_writer.write_uint64(v)?;
            }
            FieldValue::I128(v) => {
                // DLT doesn't support i128, convert to string
                log_writer.write_string(&v.to_string())?;
            }
            FieldValue::U128(v) => {
                // DLT doesn't support u128, convert to string
                log_writer.write_string(&v.to_string())?;
            }
            FieldValue::F64(v) => {
                log_writer.write_float64(v)?;
            }
            FieldValue::Bool(v) => {
                log_writer.write_bool(v)?;
            }
            FieldValue::Str(s) | FieldValue::Debug(s) => {
                log_writer.write_string(&s)?;
            }
        }
    }
    Ok(())
}

/// Typed field value that preserves the original data type
#[derive(Debug, Clone)]
enum FieldValue {
    Str(String),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F64(f64),
    Bool(bool),
    Debug(String),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::I64(v) => write!(f, "{v}"),
            FieldValue::U64(v) => write!(f, "{v}"),
            FieldValue::I128(v) => write!(f, "{v}"),
            FieldValue::U128(v) => write!(f, "{v}"),
            FieldValue::F64(v) => write!(f, "{v}"),
            FieldValue::Bool(v) => write!(f, "{v}"),
            FieldValue::Str(s) => write!(f, "\"{s}\""),
            FieldValue::Debug(s) => write!(f, "{s}"),
        }
    }
}

/// Helper visitor for extracting span/event fields with type preservation
struct FieldVisitor {
    fields: IndexMap<String, FieldValue>,
}

impl FieldVisitor {
    fn new() -> Self {
        FieldVisitor {
            fields: IndexMap::new(),
        }
    }
}

impl tracing::field::Visit for FieldVisitor {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), FieldValue::F64(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), FieldValue::I64(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), FieldValue::U64(value));
    }

    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.fields
            .insert(field.name().to_string(), FieldValue::I128(value));
    }

    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.fields
            .insert(field.name().to_string(), FieldValue::U128(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), FieldValue::Bool(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), FieldValue::Str(value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            FieldValue::Debug(format!("{value:?}")),
        );
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use super::*;

    #[tokio::test]
    async fn test_dlt_appender() {
        // This works even without a running DLT daemon because
        // the DLT C library buffers messages internally.
        let app_id = DltId::new(b"APP").unwrap();

        tracing_subscriber::registry()
            .with(DltLayer::new(&app_id, "test").expect("Failed to create DLT layer"))
            .init();

        let outer_span = tracing::info_span!("outer", level = 0);
        let _outer_entered = outer_span.enter();

        let inner_span = tracing::error_span!("inner", level = 1);
        let _inner_entered = inner_span.enter();

        tracing::info!(a_bool = true, answer = 42, message = "first example");
    }
}
