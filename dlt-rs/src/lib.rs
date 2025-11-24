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

//! Low-level Rust bindings for COVESA DLT (Diagnostic Log and Trace)
//!
//! Safe Rust API for the DLT C library with RAII semantics, enabling applications to send
//! diagnostic logs and traces to the DLT daemon for centralized logging and analysis.
//!
//! # Quick Start
//!
//! ```no_run
//! use dlt_rs::{DltApplication, DltId, DltLogLevel};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Register application (one per process)
//! let app = DltApplication::register(&DltId::new(b"MBTI")?, "Measurement & Bus Trace Interface")?;
//! let ctx = app.create_context(&DltId::new(b"MEAS")?, "Measurement Context")?;
//!
//! // Simple logging
//! ctx.log(DltLogLevel::Info, "Hello DLT!")?;
//!
//! // Structured logging with typed fields
//! let mut writer = ctx.log_write_start(DltLogLevel::Info)?;
//! writer.write_string("Temperature:")?
//!     .write_float32(87.5)?
//!     .write_string("°C")?;
//! writer.finish()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Core Types
//!
//! - [`DltApplication`] - Application registration
//! - [`DltContextHandle`] - Context for logging with specific ID
//! - [`DltLogWriter`] - Builder for structured multi-field messages
//! - [`DltLogLevel`] - Log severity (Fatal, Error, Warn, Info, Debug, Verbose)
//! - [`DltId`] - Type-safe 1-4 byte ASCII identifiers
//!
//! # Features
//!
//! - **RAII cleanup** - Automatic resource management
//! - **Structured logging** - Structured logging messages via [`DltLogWriter`]
//! - **Dynamic control** - Runtime log level changes via
//!   [`DltContextHandle::register_log_level_changed_listener()`]
//! - **Thread-safe** - All types are `Send + Sync`
//!
//! # Log Level Control
//!
//! DLT log levels can be changed at runtime by the DLT daemon or other tools.
//! Applications can listen for log level changes.
//! See [`DltLogLevel`] for all available levels and [`LogLevelChangedEvent`] to listen for changes.
//!
//! # See Also
//!
//! - [COVESA DLT](https://github.com/COVESA/dlt-daemon)
use std::{
    collections::HashMap,
    ffi::CString,
    ptr,
    sync::{Arc, OnceLock, RwLock, atomic::AtomicBool},
};

use thiserror::Error;
use tokio::sync::broadcast;

#[rustfmt::skip]
#[allow(clippy::all,
    dead_code,
    warnings,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
)]
pub use dlt_sys::{DLT_ID_SIZE, DltContext, DltContextData};

/// DLT log level
///
/// Severity level of a log message, ordered from most severe ([`DltLogLevel::Fatal`])
/// to least severe ([`DltLogLevel::Verbose`]).
///
/// Use with [`DltContextHandle::log()`] or [`DltContextHandle::log_write_start()`].
/// The DLT daemon filters messages based on the configured threshold
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DltLogLevel {
    /// Default log level (determined by DLT daemon configuration)
    Default,
    /// Logging is disabled
    Off,
    /// Fatal system error - system is unusable
    Fatal,
    /// Error conditions - operation failed
    Error,
    /// Warning conditions - something unexpected but recoverable
    Warn,
    /// Informational messages - normal operation (default level)
    Info,
    /// Debug-level messages - detailed diagnostic information
    Debug,
    /// Verbose/trace-level messages - very detailed execution traces
    Verbose,
}

impl From<i32> for DltLogLevel {
    fn from(value: i32) -> Self {
        match value {
            dlt_sys::DltLogLevelType_DLT_LOG_OFF => DltLogLevel::Off,
            dlt_sys::DltLogLevelType_DLT_LOG_FATAL => DltLogLevel::Fatal,
            dlt_sys::DltLogLevelType_DLT_LOG_ERROR => DltLogLevel::Error,
            dlt_sys::DltLogLevelType_DLT_LOG_WARN => DltLogLevel::Warn,
            dlt_sys::DltLogLevelType_DLT_LOG_INFO => DltLogLevel::Info,
            dlt_sys::DltLogLevelType_DLT_LOG_DEBUG => DltLogLevel::Debug,
            dlt_sys::DltLogLevelType_DLT_LOG_VERBOSE => DltLogLevel::Verbose,
            _ => DltLogLevel::Default,
        }
    }
}

impl From<DltLogLevel> for i32 {
    fn from(value: DltLogLevel) -> Self {
        match value {
            DltLogLevel::Default => dlt_sys::DltLogLevelType_DLT_LOG_DEFAULT,
            DltLogLevel::Off => dlt_sys::DltLogLevelType_DLT_LOG_OFF,
            DltLogLevel::Fatal => dlt_sys::DltLogLevelType_DLT_LOG_FATAL,
            DltLogLevel::Error => dlt_sys::DltLogLevelType_DLT_LOG_ERROR,
            DltLogLevel::Warn => dlt_sys::DltLogLevelType_DLT_LOG_WARN,
            DltLogLevel::Info => dlt_sys::DltLogLevelType_DLT_LOG_INFO,
            DltLogLevel::Debug => dlt_sys::DltLogLevelType_DLT_LOG_DEBUG,
            DltLogLevel::Verbose => dlt_sys::DltLogLevelType_DLT_LOG_VERBOSE,
        }
    }
}

/// Internal error types for Rust-side operations (not from libdlt)
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DltError {
    #[error("Data cannot be converted to a DLT compatible string: {0}")]
    InvalidString(String),
    #[error("Failed to register DLT context")]
    ContextRegistrationFailed,
    #[error("Failed to register DLT application")]
    ApplicationRegistrationFailed(String),
    #[error("A pointer or memory is invalid")]
    InvalidMemory,
    #[error("Failed to acquire a lock")]
    BadLock,
    #[error("Input value is invalid")]
    InvalidInput,
}

/// DLT return value error types (from libdlt C library)
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DltSysError {
    #[cfg(feature = "trace_load_ctrl")]
    #[error("DLT load exceeded")]
    /// Only available with the `trace_load_ctrl` feature enabled.
    LoadExceeded,

    #[error("DLT file size error")]
    FileSizeError,

    #[error("DLT logging disabled")]
    LoggingDisabled,

    #[error("DLT user buffer full")]
    UserBufferFull,

    #[error("DLT wrong parameter")]
    WrongParameter,

    #[error("DLT buffer full")]
    BufferFull,

    #[error("DLT pipe full")]
    PipeFull,

    #[error("DLT pipe error")]
    PipeError,

    #[error("DLT general error")]
    Error,

    #[error("DLT unknown error")]
    Unknown,
}

impl DltSysError {
    fn from_return_code(code: i32) -> Result<(), Self> {
        #[allow(unreachable_patterns)]
        match code {
            dlt_sys::DltReturnValue_DLT_RETURN_TRUE
            | dlt_sys::DltReturnValue_DLT_RETURN_OK => Ok(()),
            dlt_sys::DltReturnValue_DLT_RETURN_ERROR => Err(DltSysError::Error),
            dlt_sys::DltReturnValue_DLT_RETURN_PIPE_ERROR => Err(DltSysError::PipeError),
            dlt_sys::DltReturnValue_DLT_RETURN_PIPE_FULL => Err(DltSysError::PipeFull),
            dlt_sys::DltReturnValue_DLT_RETURN_BUFFER_FULL => Err(DltSysError::BufferFull),
            dlt_sys::DltReturnValue_DLT_RETURN_WRONG_PARAMETER => {
                Err(DltSysError::WrongParameter)
            }
            dlt_sys::DltReturnValue_DLT_RETURN_USER_BUFFER_FULL => {
                Err(DltSysError::UserBufferFull)
            }
            dlt_sys::DltReturnValue_DLT_RETURN_LOGGING_DISABLED => {
                Err(DltSysError::LoggingDisabled)
            }
            dlt_sys::DltReturnValue_DLT_RETURN_FILESZERR => Err(DltSysError::FileSizeError),
            #[cfg(feature = "trace_load_ctrl")]
            dlt_sys::DltReturnValue_DLT_RETURN_LOAD_EXCEEDED => Err(DltSysError::LoadExceeded),
            _ => Err(DltSysError::Unknown),
        }
    }
}

/// Size of DLT ID fields (Application ID, Context ID) - re-exported from bindings as usize
pub const DLT_ID_SIZE_USIZE: usize = DLT_ID_SIZE as usize;

/// A DLT identifier (Application ID or Context ID)
///
/// DLT IDs are 1-4 ASCII bytes. Create with `DltId::new(b"APP")?`.
/// Shorter IDs are internally padded with nulls
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DltId {
    bytes: [u8; DLT_ID_SIZE_USIZE],
    len: usize,
}

impl DltId {
    /// Create a new DLT ID from a byte slice of 1 to 4 bytes
    ///
    /// The ID will be validated as ASCII.
    /// IDs shorter than 4 bytes are right-padded with null bytes internally.
    ///
    /// # Errors
    /// Returns [`DltError::InvalidInput`] if the byte slice is empty, longer than 4 bytes,
    /// or contains non-ASCII characters.
    ///
    /// # Examples
    /// ```no_run
    /// # use dlt_rs::{DltId, DltError};
    /// # fn main() -> Result<(), DltError> {
    /// let id = DltId::new(b"APP")?;
    /// assert_eq!(id.as_str()?, "APP");
    ///
    /// // Too long
    /// assert!(DltId::new(b"TOOLONG").is_err());
    ///
    /// // Empty
    /// assert!(DltId::new(b"").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<const N: usize>(bytes: &[u8; N]) -> Result<Self, DltError> {
        // Validate that N is between 1 and 4
        if N == 0 || N > DLT_ID_SIZE_USIZE {
            return Err(DltError::InvalidInput);
        }

        // Validate ASCII
        if !bytes.is_ascii() {
            return Err(DltError::InvalidInput);
        }

        let mut padded = [0u8; DLT_ID_SIZE_USIZE];
        // Indexing is safe here: loop condition ensures i < N, and N <= DLT_ID_SIZE by validation
        #[allow(clippy::indexing_slicing)]
        padded[..N].copy_from_slice(&bytes[..N]);

        Ok(Self {
            bytes: padded,
            len: N,
        })
    }

    /// Get the ID as a string slice
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid UFT-8.
    /// This should never happen due to construction constraints.
    pub fn as_str(&self) -> Result<&str, DltError> {
        let slice = self
            .bytes
            .get(..self.len)
            .ok_or_else(|| DltError::InvalidString("Invalid length".to_string()))?;
        let s = std::str::from_utf8(slice).map_err(|e| DltError::InvalidString(e.to_string()))?;
        Ok(s)
    }
}

/// DLT trace status
///
/// Controls whether network trace messages (like packet captures) are enabled.
/// This is separate from log levels. Most applications only use log levels and can
/// ignore trace status.
///
/// Trace status is included in [`LogLevelChangedEvent`] notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DltTraceStatus {
    /// Use default trace status from DLT daemon configuration
    Default,
    /// Trace messages are disabled
    Off,
    /// Trace messages are enabled
    On,
}

impl From<i32> for DltTraceStatus {
    fn from(value: i32) -> Self {
        match value {
            dlt_sys::DltTraceStatusType_DLT_TRACE_STATUS_OFF => DltTraceStatus::Off,
            dlt_sys::DltTraceStatusType_DLT_TRACE_STATUS_ON => DltTraceStatus::On,
            _ => DltTraceStatus::Default,
        }
    }
}

impl From<DltTraceStatus> for i32 {
    fn from(value: DltTraceStatus) -> Self {
        match value {
            DltTraceStatus::Default => dlt_sys::DltTraceStatusType_DLT_TRACE_STATUS_DEFAULT,
            DltTraceStatus::Off => dlt_sys::DltTraceStatusType_DLT_TRACE_STATUS_OFF,
            DltTraceStatus::On => dlt_sys::DltTraceStatusType_DLT_TRACE_STATUS_ON,
        }
    }
}

/// Event sent when DLT log level or trace status changes
///
/// Emitted when the DLT daemon changes the log level or trace status for a context.
///
/// Register a listener with [`DltContextHandle::register_log_level_changed_listener()`]
/// to receive these events
#[derive(Debug, Clone, Copy)]
pub struct LogLevelChangedEvent {
    /// The DLT context ID that this change applies to
    pub context_id: DltId,
    /// The new log level for the context
    pub log_level: DltLogLevel,
    /// The new trace status for the context
    pub trace_status: DltTraceStatus,
}

struct LogLevelChangedBroadcaster {
    sender: broadcast::Sender<LogLevelChangedEvent>,
    receiver: broadcast::Receiver<LogLevelChangedEvent>,
}

// Global registry for log level change callbacks
static CALLBACK_REGISTRY: OnceLock<RwLock<HashMap<DltId, LogLevelChangedBroadcaster>>> =
    OnceLock::new();

static APP_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Internal C callback that forwards to the Rust channel
unsafe extern "C" fn internal_log_level_callback(
    context_id: *mut std::os::raw::c_char,
    log_level: u8,
    trace_status: u8,
) {
    if context_id.is_null() {
        return;
    }

    let Some(registry) = CALLBACK_REGISTRY.get() else {
        return;
    };

    let id = unsafe {
        let mut ctx_id = [0u8; DLT_ID_SIZE_USIZE];
        ptr::copy(
            context_id.cast::<u8>(),
            ctx_id.as_mut_ptr(),
            DLT_ID_SIZE_USIZE,
        );
        match DltId::new(&ctx_id) {
            Ok(id) => id,
            Err(_) => return, // Invalid context ID from DLT daemon
        }
    };

    let Ok(lock) = registry.read() else {
        return;
    };

    let Some(broadcaster) = lock.get(&id) else {
        return;
    };

    let event = LogLevelChangedEvent {
        context_id: id,
        log_level: DltLogLevel::from(i32::from(log_level)),
        trace_status: DltTraceStatus::from(i32::from(trace_status)),
    };

    let _ = broadcaster.sender.send(event);
}

/// Internal shared state for the DLT application
///
/// This ensures contexts can keep the application alive through reference counting.
/// When the last reference (either from `DltApplication` or `DltContextHandle`) is
/// dropped, the application is automatically unregistered from DLT.
struct DltApplicationHandle {
    _private: (),
}

impl Drop for DltApplicationHandle {
    fn drop(&mut self) {
        unsafe {
            // unregister from dlt, but ignore errors
            dlt_sys::unregisterApplicationFlushBufferedLogs();
            dlt_sys::dltFree();
            APP_REGISTERED.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

/// Singleton guard for DLT application registration
///
/// Only one DLT application can be registered per process. Automatically unregistered
/// when dropped and a new application can be registered.
///
/// **Lifetime Guarantee**: Contexts maintain an internal reference, keeping the application
/// registered. Safe to drop the application handle before contexts.
///
/// Cheaply cloneable for sharing across threads
pub struct DltApplication {
    inner: Arc<DltApplicationHandle>,
}

impl DltApplication {
    /// Register a DLT application
    ///
    /// Only one application can be registered per process. If you need to register
    /// a different application, drop this instance first.
    ///
    /// The returned handle can be cloned to share the application across threads.
    ///
    /// # Errors
    /// Returns `DltError` if the registration fails
    pub fn register(app_id: &DltId, app_description: &str) -> Result<Self, DltError> {
        if APP_REGISTERED
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            return Err(DltError::ApplicationRegistrationFailed(
                "An application is already registered in this process".to_string(),
            ));
        }

        let app_id_str = app_id.as_str()?;
        let app_id_c = CString::new(app_id_str).map_err(|_| {
            DltError::InvalidString("App id could not be converted to string".to_owned())
        })?;
        let app_desc_c = CString::new(app_description).map_err(|_| {
            DltError::InvalidString("Context id could not be converted to string".to_owned())
        })?;

        unsafe {
            let ret = dlt_sys::registerApplication(app_id_c.as_ptr(), app_desc_c.as_ptr());
            DltSysError::from_return_code(ret).map_err(|_| {
                DltError::ApplicationRegistrationFailed(format!(
                    "Failed to register application: {ret}"
                ))
            })?;
        }
        Ok(DltApplication {
            inner: Arc::new(DltApplicationHandle { _private: () }),
        })
    }

    /// Create a new DLT context within this application
    ///
    /// The created context maintains an internal reference to the application,
    /// ensuring the application remains registered as long as the context exists.
    ///
    /// # Errors
    /// Returns `DltError` if registration fails
    pub fn create_context(
        &self,
        context_id: &DltId,
        context_description: &str,
    ) -> Result<DltContextHandle, DltError> {
        DltContextHandle::new(context_id, context_description, Arc::clone(&self.inner))
    }
}

impl Clone for DltApplication {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Safe to send between threads (application registration is process-wide)
unsafe impl Send for DltApplication {}
unsafe impl Sync for DltApplication {}

/// Safe wrapper around C DLT context with RAII semantics
///
/// The context holds an internal reference to the application, ensuring the
/// application remains registered as long as any context exists.
pub struct DltContextHandle {
    context: *mut DltContext,
    _app: Arc<DltApplicationHandle>,
}

impl DltContextHandle {
    /// Register a new DLT context
    ///
    /// # Errors
    /// Returns `DltError` if registration fails
    fn new(
        context_id: &DltId,
        context_description: &str,
        app: Arc<DltApplicationHandle>,
    ) -> Result<Self, DltError> {
        let context_id_str = context_id.as_str()?;
        let ctx_id_c = CString::new(context_id_str)
            .map_err(|_| DltError::InvalidString("Context ID is not a valid string".to_owned()))?;
        let ctx_desc_c = CString::new(context_description).map_err(|_| {
            DltError::InvalidString("Context description is not a valid string".to_owned())
        })?;

        unsafe {
            let context = dlt_sys::registerContext(ctx_id_c.as_ptr(), ctx_desc_c.as_ptr());
            if context.is_null() {
                Err(DltError::ContextRegistrationFailed)
            } else {
                Ok(DltContextHandle { context, _app: app })
            }
        }
    }

    fn raw_context(&self) -> Result<DltContext, DltError> {
        let context = unsafe {
            if self.context.is_null() {
                return Err(DltError::ContextRegistrationFailed);
            }
            *self.context
        };
        Ok(context)
    }

    /// Get the context ID
    /// # Errors
    /// Returns `DltError` if the context is invalid or the context is null
    pub fn context_id(&self) -> Result<DltId, DltError> {
        let ctx_id = unsafe {
            // this is a false positive of clippy.
            // raw_context.contextID is of type [::std::os::raw::c_char; 4usize], which
            // cannot be directly used as &[u8; 4].
            #[allow(clippy::useless_transmute)]
            std::mem::transmute::<[std::os::raw::c_char; 4], [u8; 4]>(self.raw_context()?.contextID)
        };
        DltId::new(&ctx_id)
    }

    /// Get the current trace status of the context
    /// # Errors
    /// Returns `DltError` if the context is invalid or the context is null
    #[must_use]
    pub fn trace_status(&self) -> DltTraceStatus {
        self.raw_context()
            .ok()
            .and_then(|rc| {
                if rc.log_level_ptr.is_null() {
                    None
                } else {
                    Some(DltTraceStatus::from(i32::from(unsafe {
                        *rc.trace_status_ptr
                    })))
                }
            })
            .unwrap_or(DltTraceStatus::Default)
    }

    /// Get the current log level of the context
    #[must_use]
    pub fn log_level(&self) -> DltLogLevel {
        self.raw_context()
            .ok()
            .and_then(|rc| {
                if rc.log_level_ptr.is_null() {
                    None
                } else {
                    Some(DltLogLevel::from(i32::from(unsafe { *rc.log_level_ptr })))
                }
            })
            .unwrap_or(DltLogLevel::Default)
    }

    /// Log a simple string message
    ///
    /// # Errors
    /// Returns `DltError` if logging fails
    pub fn log(&self, log_level: DltLogLevel, message: &str) -> Result<(), DltSysError> {
        let msg_c = CString::new(message).map_err(|_| DltSysError::WrongParameter)?;

        unsafe {
            let ret = dlt_sys::logDlt(self.context, log_level.into(), msg_c.as_ptr());
            DltSysError::from_return_code(ret)
        }
    }

    /// Start a complex log message with a custom timestamp.
    /// Can be used to hide original timestamps or to log event recorded earlier.
    /// The timestamp is a steady clock, starting from an arbitrary point in time,
    /// usually system start.
    ///
    /// # Errors
    /// Returns `DltError` if starting the log message fails
    pub fn log_write_start_custom_timestamp(
        &self,
        log_level: DltLogLevel,
        timestamp_microseconds: u64,
    ) -> Result<DltLogWriter, DltSysError> {
        let mut log_writer = self.log_write_start(log_level)?;
        // timestamp resolution in dlt is .1 milliseconds.
        let timestamp =
            u32::try_from(timestamp_microseconds / 100).map_err(|_| DltSysError::WrongParameter)?;
        log_writer.log_data.use_timestamp = dlt_sys::DltTimestampType_DLT_USER_TIMESTAMP;
        log_writer.log_data.user_timestamp = timestamp;
        Ok(log_writer)
    }

    /// Start a complex log message
    ///
    /// # Errors
    /// Returns `DltError` if starting the log message fails
    pub fn log_write_start(&self, log_level: DltLogLevel) -> Result<DltLogWriter, DltSysError> {
        let mut log_data = DltContextData::default();

        unsafe {
            let ret = dlt_sys::dltUserLogWriteStart(
                self.context,
                &raw mut log_data,
                log_level.into(),
            );

            DltSysError::from_return_code(ret)?;
            Ok(DltLogWriter { log_data })
        }
    }

    /// Register a channel to receive log level change notifications
    ///
    /// Returns a receiver that will get `LogLevelChangeEvent`
    /// when the DLT daemon changes log levels
    ///
    /// # Errors
    /// Returns `InternalError` if callback registration with DLT fails
    pub fn register_log_level_changed_listener(
        &self,
    ) -> Result<broadcast::Receiver<LogLevelChangedEvent>, DltError> {
        let rwlock = CALLBACK_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        let mut guard = rwlock.write().map_err(|_| DltError::BadLock)?;
        let ctx_id = self.context_id()?;

        if let Some(broadcaster) = guard.get_mut(&ctx_id) {
            Ok(broadcaster.receiver.resubscribe())
        } else {
            unsafe {
                let ret = dlt_sys::registerLogLevelChangedCallback(
                    self.context,
                    Some(internal_log_level_callback),
                );
                DltSysError::from_return_code(ret)
                    .map_err(|_| DltError::ContextRegistrationFailed)?;
            }
            let (tx, rx) = broadcast::channel(5);
            let rx_clone = rx.resubscribe();
            guard.insert(
                ctx_id,
                LogLevelChangedBroadcaster {
                    sender: tx,
                    receiver: rx,
                },
            );
            Ok(rx_clone)
        }
    }
}

impl Drop for DltContextHandle {
    fn drop(&mut self) {
        let context_id = self.context_id();
        if let Some(lock) = CALLBACK_REGISTRY.get()
            && let Ok(mut guard) = lock.write()
            && let Ok(ctx_id) = context_id
        {
            guard.remove(&ctx_id);
        }

        unsafe {
            dlt_sys::unregisterContext(self.context);
        }
    }
}

// Safe to send between threads, per DLT documentation
unsafe impl Send for DltContextHandle {}
unsafe impl Sync for DltContextHandle {}

/// Builder for structured log messages with multiple typed fields
///
/// Construct log messages with typed data fields sent in binary format for efficiency.
/// Each field retains type information for proper display in DLT viewers.
///
/// # Usage
///
/// 1. Start with [`DltContextHandle::log_write_start()`]
/// 2. Chain `write_*` methods to add fields
/// 3. Call [`finish()`](DltLogWriter::finish()) to send
///
/// Auto-finishes on drop if [`finish()`](DltLogWriter::finish()) not called (errors ignored).
///
/// # Example
///
/// ```no_run
/// # use dlt_rs::{DltApplication, DltId, DltLogLevel};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let app = DltApplication::register(&DltId::new(b"MBTI")?,
///     "Measurement and Bus Trace Interface")?;
/// # let ctx = app.create_context(&DltId::new(b"MEAS")?, "Context")?;
/// let mut writer = ctx.log_write_start(DltLogLevel::Info)?;
/// writer.write_string("Temperature:")?
///    .write_float32(87.5)?
///    .write_string("°C")?;
/// writer.finish()?;
/// # Ok(())
/// # }
/// ```
///
/// # Available Methods
///
/// [`write_string()`](DltLogWriter::write_string()) |
/// [`write_i32()`](DltLogWriter::write_i32()) |
/// [`write_u32()`](DltLogWriter::write_u32()) |
/// [`write_int64()`](DltLogWriter::write_int64()) |
/// [`write_uint64()`](DltLogWriter::write_uint64()) |
/// [`write_float32()`](DltLogWriter::write_float32()) |
/// [`write_float64()`](DltLogWriter::write_float64()) |
/// [`write_bool()`](DltLogWriter::write_bool())
pub struct DltLogWriter {
    log_data: DltContextData,
}

impl DltLogWriter {
    /// Write a string to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_string(&mut self, text: &str) -> Result<&mut Self, DltSysError> {
        let text_c = CString::new(text).map_err(|_| DltSysError::WrongParameter)?;

        unsafe {
            let ret = dlt_sys::dltUserLogWriteString(&raw mut self.log_data, text_c.as_ptr());
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write an unsigned integer to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_u32(&mut self, value: u32) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteUint(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write a signed integer to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_i32(&mut self, value: i32) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteInt(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write an unsigned 64-bit integer to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_uint64(&mut self, value: u64) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteUint64(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write a signed 64-bit integer to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_int64(&mut self, value: i64) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteInt64(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write a 32-bit float to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_float32(&mut self, value: f32) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteFloat32(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write a 64-bit float to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_float64(&mut self, value: f64) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteFloat64(&raw mut self.log_data, value);
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Write a boolean to the log message
    ///
    /// # Errors
    /// Returns `DltError` if writing fails
    pub fn write_bool(&mut self, value: bool) -> Result<&mut Self, DltSysError> {
        unsafe {
            let ret = dlt_sys::dltUserLogWriteBool(&raw mut self.log_data, u8::from(value));
            DltSysError::from_return_code(ret)?;
        }

        Ok(self)
    }

    /// Finish and send the log message
    ///
    /// Explicitly finishes the log message. If not called, the message will be
    /// automatically finished when the `DltLogWriter` is dropped, but errors will be ignored.
    ///
    /// # Errors
    /// Returns `DltError` if finishing fails
    pub fn finish(mut self) -> Result<(), DltSysError> {
        let ret = unsafe { dlt_sys::dltUserLogWriteFinish(&raw mut self.log_data) };
        // Prevent Drop from running since we've already finished
        std::mem::forget(self);
        DltSysError::from_return_code(ret)
    }
}

impl Drop for DltLogWriter {
    fn drop(&mut self) {
        // Auto-finish the log message if finish() wasn't called explicitly
        unsafe {
            let _ = dlt_sys::dltUserLogWriteFinish(&raw mut self.log_data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlt_error_from_return_code() {
        assert!(DltSysError::from_return_code(0).is_ok());
        assert!(DltSysError::from_return_code(1).is_ok());
        assert_eq!(DltSysError::from_return_code(-1), Err(DltSysError::Error));
        assert_eq!(
            DltSysError::from_return_code(-5),
            Err(DltSysError::WrongParameter)
        );
    }

    #[test]
    fn test_dlt_id_creation() {
        // 1 byte ID
        let short_id = DltId::new(b"A").unwrap();
        assert_eq!(short_id.as_str().unwrap(), "A");

        // 3 byte IDs
        let app_id = DltId::new(b"APP").unwrap();
        assert_eq!(app_id.as_str().unwrap(), "APP");

        let ctx_id = DltId::new(b"CTX").unwrap();
        assert_eq!(ctx_id.as_str().unwrap(), "CTX");

        // 4 byte ID (maximum)
        let full_id = DltId::new(b"ABCD").unwrap();
        assert_eq!(full_id.as_str().unwrap(), "ABCD");
    }

    #[test]
    fn test_dlt_id_too_long() {
        let result = DltId::new(b"TOOLONG");
        assert_eq!(result.unwrap_err(), DltError::InvalidInput);
    }

    #[test]
    fn test_dlt_id_empty() {
        let result = DltId::new(b"");
        assert_eq!(result.unwrap_err(), DltError::InvalidInput);
    }

    #[test]
    fn test_dlt_id_non_ascii() {
        let result = DltId::new(b"\xFF\xFE");
        assert_eq!(result.unwrap_err(), DltError::InvalidInput);
    }

    #[test]
    fn test_dlt_id_equality() {
        let id1 = DltId::new(b"APP").unwrap();
        let id2 = DltId::new(b"APP").unwrap();
        let id3 = DltId::new(b"CTX").unwrap();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Different lengths are not equal
        let id4 = DltId::new(b"A").unwrap();
        assert_ne!(id1, id4);
    }
}
