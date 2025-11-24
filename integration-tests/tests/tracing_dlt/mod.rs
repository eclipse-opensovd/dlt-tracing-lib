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
use std::{sync::Arc, time::Duration};

use serial_test::serial;
use tracing_dlt::{DltApplication, DltId, DltLayer, DltLogLevel};
use tracing_subscriber::{Registry, layer::SubscriberExt};

use crate::{DltReceiver, assert_contains, change_dlt_log_level, ensure_dlt_daemon_running};

struct LogFile {
    path: std::path::PathBuf,
}

impl Drop for LogFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct TracingGuard {
    _guard: tracing::subscriber::DefaultGuard,
    log_file: Option<LogFile>,
    dlt_app: Arc<DltApplication>,
}

#[allow(clippy::unwrap_used)]
fn init_tracing(with_file: bool) -> TracingGuard {
    ensure_dlt_daemon_running();
    let app_id = DltId::new(b"TEST").unwrap();
    let dlt_layer = DltLayer::new(&app_id, "Default Context Test").unwrap();
    let dlt_app = Arc::clone(&dlt_layer.app);

    let (guard, log_file) = if with_file {
        let log_file =
            std::env::temp_dir().join(format!("test_console_output_{}.log", std::process::id()));
        let file = std::fs::File::create(&log_file).unwrap();

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false);

        let subscriber = Registry::default().with(dlt_layer).with(file_layer);
        let guard = tracing::subscriber::set_default(subscriber);
        (guard, Some(LogFile { path: log_file }))
    } else {
        let subscriber = Registry::default().with(dlt_layer);
        let guard = tracing::subscriber::set_default(subscriber);
        (guard, None)
    };

    TracingGuard {
        _guard: guard,
        log_file,
        dlt_app,
    }
}

#[tokio::test]
#[serial]
async fn test_tracing_to_dlt() {
    let _guard = init_tracing(false);
    let receiver = DltReceiver::start();

    // Log some messages
    tracing::info!("Test info message");
    tracing::warn!("Test warning message");
    tracing::error!("Test error message");
    // Give time for messages to be processed
    tokio::time::sleep(Duration::from_millis(200)).await;
    let output = receiver.stop_and_get_output();
    for expected_string in [
        "TEST DFLT log info V 1 [lib::tracing_dlt: Test info message]",
        "TEST DFLT log warn V 1 [lib::tracing_dlt: Test warning message]",
        "TEST DFLT log error V 1 [lib::tracing_dlt: Test error message]",
    ] {
        assert_contains(&output, expected_string);
    }
}

#[tokio::test]
#[serial]
async fn test_with_spans_and_context_id() {
    let _guard = init_tracing(false);
    let receiver = DltReceiver::start();

    {
        let outer_span = tracing::span!(
            tracing::Level::INFO,
            "outer_span",
            task = "processing",
            dlt_context = "CONTEXT_TOO_LONG"
        );
        let _outer_guard = outer_span.enter();
        tracing::info!("Inside outer with context too long");
        {
            let inner_span = tracing::span!(
                tracing::Level::INFO,
                "inner_span",
                step = 1,
                dlt_context = "CTX1"
            );
            let _inner_guard = inner_span.enter();
            tracing::info!("Inside inner span");
        }
        tracing::info!("Back in outer span");
    }
    tracing::info!("default context");

    // Verify DLT output, make sure the file appender is also not writing the empty context
    let dlt_output = receiver.stop_and_get_output();
    let outer = r#"outer_span{task="processing"}"#;
    let inner = "inner_span{step=1}";
    let target = "lib::tracing_dlt";
    for expected_string in [
        format!("TEST CONT log info V 2 [{outer}: {target}: Inside outer with context too long"),
        format!("TEST CTX1 log info V 2 [{outer}:{inner}: {target}: Inside inner span]"),
        // the dlt_context is still set, because this logs belongs to the outer span
        format!("TEST CONT log info V 2 [{outer}: {target}: Back in outer span]"),
        format!("TEST DFLT log info V 1 [{target}: default context]"),
    ] {
        assert_contains(&dlt_output, &expected_string);
    }
}

#[tokio::test]
#[serial]
async fn test_tracing_with_default_context() {
    let guard = init_tracing(true);
    let receiver = DltReceiver::start();

    // Create span without dlt_context field - should use default context
    // The dlt_context is set to None::<&str>, to test if the empty field is omitted even if
    // it's present but empty
    let outer_span = tracing::span!(
        tracing::Level::INFO,
        "outer_span",
        task = "processing",
        dlt_context = None::<&str>
    );
    let _outer_guard = outer_span.enter();
    tracing::info!("Inside outer span");
    {
        let inner_span = tracing::span!(tracing::Level::INFO, "inner_span", step = 1);
        let _inner_guard = inner_span.enter();
        tracing::info!("inner");
    }
    tracing::info!("Back in outer span");

    // Verify DLT output, make sure the file appender is also not writing the empty context
    let dlt_output = receiver.stop_and_get_output();
    let log_file_path = &guard.log_file.as_ref().expect("Log file should exist").path;
    let console_output = std::fs::read_to_string(log_file_path).expect("Failed to read log file");

    for expected_string in [
        r#"outer_span{task="processing"}: lib::tracing_dlt: Inside outer span"#,
        r#"outer_span{task="processing"}:inner_span{step=1}: lib::tracing_dlt: inner"#,
        r#"outer_span{task="processing"}: lib::tracing_dlt: Back in outer span"#,
    ] {
        assert_contains(&dlt_output, expected_string);
        assert_contains(&console_output, expected_string);
    }
}

#[tokio::test]
#[serial]
async fn test_concurrent_logging() {
    let _guard = init_tracing(false);
    let receiver = DltReceiver::start();
    // Spawn multiple tasks that log concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                tracing::info!(task = i, iteration = j, "Concurrent log message");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
    }
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let messages = receiver.stop_and_get_output();
    // Verify that all messages are present
    for i in 0..5 {
        for j in 0..10 {
            let expected = format!(
                "TEST DFLT log info V 7 [lib::tracing_dlt: Concurrent log message task = {i} \
                 iteration = {j}]",
            );
            assert_contains(&messages, &expected);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_debug_logs() {
    let _guard = init_tracing(false);
    let receiver = DltReceiver::start();

    let not_shown_msg = "this debug log should not appear";
    tracing::debug!("{}", not_shown_msg);

    change_dlt_log_level(DltLogLevel::Debug, None, None);
    let shown_msg = "now it shows up";
    tracing::debug!(shown_msg);

    let output = receiver.stop_and_get_output();
    assert!(
        !output.contains(not_shown_msg),
        "Debug log appeared before log level change"
    );
    assert_contains(&output, shown_msg);
}

#[tokio::test]
#[serial]
async fn test_mixed_tracing_and_low_level_dlt() {
    // Initialize tracing layer
    let guard = init_tracing(false);
    let receiver = DltReceiver::start();

    // Use tracing API (high-level)
    tracing::info!("Message from tracing API");
    tracing::warn!(component = "sensor", "Tracing warning with field");

    // Create a low-level DLT context with a different context ID
    let low_level_ctx = guard
        .dlt_app
        .create_context(&DltId::new(b"LLVL").unwrap(), "Low Level Context")
        .unwrap();

    // Use low-level DLT API
    low_level_ctx
        .log(DltLogLevel::Info, "Message from low-level DLT")
        .unwrap();

    // Use structured logging with low-level API
    let mut writer = low_level_ctx.log_write_start(DltLogLevel::Warn).unwrap();
    writer.write_string("Temperature:").unwrap();
    writer.write_float32(42.42).unwrap();
    writer.write_string("°C").unwrap();
    writer.finish().unwrap();

    // Mix both APIs - tracing with span and low-level in between
    {
        let span = tracing::span!(tracing::Level::INFO, "processing", dlt_context = "PROC");
        let _enter = span.enter();
        tracing::info!("Inside tracing span");

        // Use low-level DLT within the span context
        low_level_ctx
            .log(DltLogLevel::Error, "Low-level error during processing")
            .unwrap();

        tracing::error!("Tracing error in same span");
    }

    // Back to default tracing context
    tracing::info!("Final message from tracing");
    let output = receiver.stop_and_get_output();

    // Verify tracing messages (using DFLT or PROC context)
    assert_contains(
        &output,
        "TEST DFLT log info V 1 [lib::tracing_dlt: Message from tracing API]",
    );
    assert_contains(
        &output,
        "TEST DFLT log warn V 4 [lib::tracing_dlt: Tracing warning with field component = sensor]",
    );

    // Verify low-level DLT messages (using LLVL context)
    assert_contains(
        &output,
        "TEST LLVL log info V 1 [Message from low-level DLT]",
    );
    assert_contains(&output, "TEST LLVL log warn V 3 [Temperature: 42.42 °C]");

    // Verify messages from within span
    assert_contains(
        &output,
        "TEST PROC log info V 2 [processing: lib::tracing_dlt: Inside tracing span]",
    );
    assert_contains(
        &output,
        "TEST LLVL log error V 1 [Low-level error during processing]",
    );
    assert_contains(
        &output,
        "TEST PROC log error V 2 [processing: lib::tracing_dlt: Tracing error in same span]",
    );

    // Verify final tracing message
    assert_contains(
        &output,
        "TEST DFLT log info V 1 [lib::tracing_dlt: Final message from tracing]",
    );
}
