// Copyright (c) 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
//! Common test utilities for DLT tracing integration tests
//!
//! This crate provides helper functions and utilities for testing DLT functionality,
//! including DLT daemon connectivity checks and message verification via dlt-receive.
use std::{
    io::Read,
    net::{SocketAddr, TcpStream},
    process::{self, Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};

use ::dlt_rs::{DltId, DltLogLevel};

mod dlt_rs;
mod tracing_dlt;

static DLT_DAEMON: OnceLock<Arc<Mutex<Option<process::Child>>>> = OnceLock::new();

#[must_use]
pub fn is_dlt_control_available() -> bool {
    Command::new("dlt-control")
        .arg("-h")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub(crate) fn change_dlt_log_level(
    level: DltLogLevel,
    app_id: Option<&DltId>,
    ctx_id: Option<&DltId>,
) {
    assert!(
        is_dlt_control_available(),
        "dlt-control is required for log-level control tests"
    );

    let level_num: i32 = level.into();

    let mut cmd = Command::new("dlt-control");
    cmd.args(["-l", &level_num.to_string()]);

    if let Some(app_id) = app_id {
        cmd.args(["-a", app_id.as_str().expect("Invalid application ID")]);
    }
    if let Some(ctx_id) = ctx_id {
        cmd.args(["-c", ctx_id.as_str().expect("Invalid context ID")]);
    }

    cmd.args(["127.0.0.1"]);

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute dlt-control");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("dlt-control failed: {stderr}");
    }

    // Give time for the log level change to take effect
    thread::sleep(Duration::from_millis(100));
}

/// Check if the DLT daemon is running by attempting to connect to it
///
/// Returns true if a connection can be established, false otherwise.
#[must_use]
pub fn is_dlt_daemon_running() -> bool {
    // Probe daemon TCP endpoint directly. This is independent of dlt-receive output format.
    let addr: SocketAddr = match "127.0.0.1:3490".parse() {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok()
}

/// Start the DLT daemon if it's not already running
///
/// This function ensures the DLT daemon is started only once for all tests.
/// It will check if a daemon is already running, and if not, start a new one.
///
/// # Panics
/// Panics if the daemon cannot be started
pub fn ensure_dlt_daemon_running() {
    let daemon_holder = DLT_DAEMON.get_or_init(|| Arc::new(Mutex::new(None)));
    let mut daemon_guard = daemon_holder
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Check if daemon is already running externally
    if is_dlt_daemon_running() {
        return;
    }

    // If we have a daemon process, but it's not responding, clean it up
    if let Some(ref mut daemon) = *daemon_guard
        && daemon.try_wait().ok().flatten().is_some()
    {
        *daemon_guard = None;
    }

    // Start daemon if we don't have one
    if daemon_guard.is_none() {
        println!("Starting DLT daemon...");
        let daemon = Command::new("dlt-daemon")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start dlt-daemon. Make sure it's installed.");

        *daemon_guard = Some(daemon);

        // Drop lock before any potential panic to avoid poisoning shared state.
        drop(daemon_guard);

        // Wait up to 5 seconds for daemon readiness.
        for _ in 0..25 {
            if is_dlt_daemon_running() {
                return;
            }
            thread::sleep(Duration::from_millis(200));
        }

        // Try to provide useful daemon stderr for diagnosis.
        let daemon_holder = DLT_DAEMON.get_or_init(|| Arc::new(Mutex::new(None)));
        let mut daemon_guard = daemon_holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut daemon_stderr = String::new();
        if let Some(ref mut daemon) = *daemon_guard
            && let Some(ref mut stderr) = daemon.stderr
        {
            let _ = stderr.read_to_string(&mut daemon_stderr);
        }
        panic!("DLT daemon started but is not responding. stderr: {daemon_stderr}");
    }
}
/// Helper for capturing and verifying DLT messages via dlt-receive
pub struct DltReceiver {
    process: process::Child,
}

impl DltReceiver {
    /// Start dlt-receive in background to capture DLT messages
    ///
    /// # Panics
    /// Panics if dlt-receive cannot be started
    #[must_use]
    pub fn start() -> Self {
        // stdbuf is used to disable output buffering for real-time capture
        let process = Command::new("stdbuf")
            .args(["-o0", "-e0", "dlt-receive", "-a", "127.0.0.1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start dlt-receive with stdbuf");

        // Give dlt-receive time to start and connect
        thread::sleep(Duration::from_millis(250));
        DltReceiver { process }
    }
    /// Stop dlt-receive and get captured output
    ///
    /// # Panics
    /// Panics if output cannot be retrieved
    #[must_use]
    pub fn stop_and_get_output(mut self) -> String {
        // Give time for messages to be processed
        thread::sleep(Duration::from_millis(250));
        // Stop dlt-receive
        assert!(self.process.kill().is_ok());
        let output = self
            .process
            .wait_with_output()
            .expect("Failed to get output");
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

/// Assert that output contains expected text
///
/// # Panics
/// Panics if the expected text is not found in the output
pub fn assert_contains(output: &str, expected: &str) {
    assert!(
        output.contains(expected),
        "Expected text '{expected}' not found in output: '{output}'",
    );
}

/// Assert that output contains all expected texts
///
/// # Panics
/// Panics if any of the expected texts is not found in the output
pub fn assert_contains_all(output: &str, expected: &[&str]) {
    for text in expected {
        assert_contains(output, text);
    }
}
