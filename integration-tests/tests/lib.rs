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
    process::{self, Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};

use ::dlt_sys::{DltId, DltLogLevel};

mod dlt_sys;
mod dlt_tracing_appender;

static DLT_DAEMON: OnceLock<Arc<Mutex<Option<process::Child>>>> = OnceLock::new();

pub(crate) fn change_dlt_log_level(
    level: DltLogLevel,
    app_id: Option<&DltId>,
    ctx_id: Option<&DltId>,
) {
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
    // Try to connect using dlt-receive with a timeout and check for successful connection message
    let output = Command::new("timeout")
        .args(["1", "dlt-receive", "-a", "127.0.0.1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("New client connection")
    } else {
        false
    }
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
    let mut daemon_guard = daemon_holder.lock().expect("Daemon lock failed");

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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start dlt-daemon. Make sure it's installed.");

        *daemon_guard = Some(daemon);

        // Give daemon time to start up
        thread::sleep(Duration::from_millis(250));

        // Verify it started successfully
        assert!(
            is_dlt_daemon_running(),
            "DLT daemon started but is not responding"
        );
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
