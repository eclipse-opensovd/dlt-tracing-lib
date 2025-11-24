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

use dlt_rs::{DltApplication, DltId, DltLogLevel, LogLevelChangedEvent};
use serial_test::serial;
use tokio::sync::broadcast;

use crate::{
    DltReceiver, assert_contains, assert_contains_all, change_dlt_log_level,
    ensure_dlt_daemon_running,
};

#[tokio::test]
#[serial]
async fn test_register_application_and_context() {
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"ITST").unwrap();
    let ctx_id = DltId::new(b"CTX1").unwrap();

    let app = DltApplication::register(&app_id, "Integration test app").unwrap();
    let context = app
        .create_context(&ctx_id, "Integration test context")
        .unwrap();

    let receiver = DltReceiver::start();

    let test_msg = "Integration test message";
    context.log(DltLogLevel::Info, test_msg).unwrap();

    // Verify message appears in DLT output
    let output = receiver.stop_and_get_output();
    assert_contains(
        &output,
        &format!(
            "{} {} log info V 1 [{test_msg}]",
            app_id.as_str().unwrap(),
            ctx_id.as_str().unwrap()
        ),
    );
}

#[tokio::test]
#[serial]
async fn test_complex_log_message() {
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"ICMP").unwrap();
    let ctx_id = DltId::new(b"CPLX").unwrap();

    let app = DltApplication::register(&app_id, "Complex log test").unwrap();
    let context = app.create_context(&ctx_id, "Complex context").unwrap();

    let receiver = DltReceiver::start();

    let mut log_writer = context
        .log_write_start(DltLogLevel::Error)
        .expect("Failed to start log");

    log_writer.write_string("Test field").unwrap();
    log_writer.write_u32(42).unwrap();
    log_writer.write_i32(-123).unwrap();
    log_writer.write_bool(true).unwrap();

    log_writer.finish().expect("Failed to finish log");

    let output = receiver.stop_and_get_output();
    assert_contains(
        &output,
        &format!(
            "{} {} log error V 4 [Test field 42 -123 1]",
            app_id.as_str().unwrap(),
            ctx_id.as_str().unwrap()
        ),
    );
}

#[tokio::test]
#[serial]
async fn test_different_log_levels() {
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"ILVL").unwrap();
    let ctx_id = DltId::new(b"LEVL").unwrap();

    let app = DltApplication::register(&app_id, "Log level test").unwrap();
    let context = app.create_context(&ctx_id, "Level context").unwrap();

    let receiver = DltReceiver::start();

    context.log(DltLogLevel::Fatal, "Fatal_unique_msg").unwrap();
    context.log(DltLogLevel::Error, "Error_unique_msg").unwrap();
    context
        .log(DltLogLevel::Warn, "Warning_unique_msg")
        .unwrap();
    context.log(DltLogLevel::Info, "Info_unique_msg").unwrap();
    context.log(DltLogLevel::Debug, "Debug_unique_msg").unwrap();
    context
        .log(DltLogLevel::Verbose, "Verbose_unique_msg")
        .unwrap();

    let output = receiver.stop_and_get_output();
    assert_contains_all(
        &output,
        &[
            "ILVL LEVL log fatal V 1 [Fatal_unique_msg]",
            "ILVL LEVL log error V 1 [Error_unique_msg]",
            "ILVL LEVL log warn V 1 [Warning_unique_msg]",
            "ILVL LEVL log info V 1 [Info_unique_msg]",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_registering_application_twice() {
    ensure_dlt_daemon_running();

    let app_id1 = DltId::new(b"APP1").unwrap();
    let app_id2 = DltId::new(b"APP2").unwrap();

    let app1 = DltApplication::register(&app_id1, "double register test first").unwrap();
    assert!(DltApplication::register(&app_id2, "double register test second").is_err());
    drop(app1);

    // after dropping the first application, registering the second one should work
    let _ = DltApplication::register(&app_id2, "double register test second").unwrap();
}

#[tokio::test]
#[serial]
async fn test_log_level_changed() {
    async fn wait_for_log_level_changed_event(
        rx: &mut broadcast::Receiver<LogLevelChangedEvent>,
    ) -> LogLevelChangedEvent {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        event
                    }
                    Err(e) => {
                        panic!("Failed to receive log level change: {e:?}");
                    }
                }
            }
            () = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => {
                panic!("Timeout waiting for log level change");
            }
        }
    }

    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"LVCH").unwrap();
    let ctx_id = DltId::new(b"CTX1").unwrap();

    let app = DltApplication::register(&app_id, "testing log level changes").unwrap();
    let context = app
        .create_context(&ctx_id, "context for log level change")
        .unwrap();
    let mut rx = context.register_log_level_changed_listener().unwrap();
    let event = wait_for_log_level_changed_event(&mut rx).await;
    assert_eq!(event.log_level, DltLogLevel::Info); // default log level, sent on registration
    assert_eq!(context.log_level(), DltLogLevel::Info);

    // change given context
    let level = DltLogLevel::Debug;
    change_dlt_log_level(level, Some(&app_id), Some(&ctx_id));
    let event = wait_for_log_level_changed_event(&mut rx).await;
    assert_eq!(event.log_level, level);
    assert_eq!(context.log_level(), level);

    // change all contexts
    let level = DltLogLevel::Error;
    change_dlt_log_level(level, Some(&app_id), None);
    let event = wait_for_log_level_changed_event(&mut rx).await;
    assert_eq!(event.log_level, level);
    assert_eq!(context.log_level(), level);

    // change all applications
    let level = DltLogLevel::Fatal;
    change_dlt_log_level(level, None, None);
    let event = wait_for_log_level_changed_event(&mut rx).await;
    assert_eq!(event.log_level, level);
    assert_eq!(context.log_level(), level);

    // change unrelated context - should not receive an event
    let other_ctx_id = DltId::new(b"CTX2").unwrap();
    change_dlt_log_level(DltLogLevel::Debug, Some(&app_id), Some(&other_ctx_id));
    assert_eq!(context.log_level(), level);
}

#[tokio::test]
#[serial]
async fn test_context_outlives_application() {
    // This test verifies the safety fix: contexts can outlive the application handle
    // without causing use-after-free, because contexts keep the application alive
    // through internal reference counting.
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"OLIV").unwrap();
    let ctx_id = DltId::new(b"CTX1").unwrap();

    let receiver = DltReceiver::start();

    // Create context, then drop application handle
    let context = {
        let app = DltApplication::register(&app_id, "Outlive test app").unwrap();
        let ctx = app.create_context(&ctx_id, "Outlive test context").unwrap();

        // Drop app explicitly - context should keep it alive
        drop(app);

        ctx // Return context, which outlives app
    };

    // Context should still be valid and able to log
    context
        .log(DltLogLevel::Info, "Context_outlived_app")
        .unwrap();

    // Verify message appears in DLT output
    let output = receiver.stop_and_get_output();
    assert_contains(
        &output,
        &format!(
            "{} {} log info V 1 [Context_outlived_app]",
            app_id.as_str().unwrap(),
            ctx_id.as_str().unwrap()
        ),
    );
}

#[tokio::test]
#[serial]
async fn test_multiple_contexts_keep_app_alive() {
    // Test that multiple contexts all keep the application alive,
    // and the application is only unregistered when all contexts are dropped
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"MULT").unwrap();
    let ctx1_id = DltId::new(b"CTX1").unwrap();
    let ctx2_id = DltId::new(b"CTX2").unwrap();

    let receiver = DltReceiver::start();

    let (ctx1, ctx2) = {
        let app = DltApplication::register(&app_id, "Multi context test").unwrap();
        let c1 = app.create_context(&ctx1_id, "Context 1").unwrap();
        let c2 = app.create_context(&ctx2_id, "Context 2").unwrap();

        // Drop app - both contexts should keep it alive
        drop(app);

        (c1, c2)
    };

    // Both contexts should work
    ctx1.log(DltLogLevel::Info, "Context1_message").unwrap();
    ctx2.log(DltLogLevel::Info, "Context2_message").unwrap();

    // Drop first context - second should still work
    drop(ctx1);
    ctx2.log(DltLogLevel::Info, "Context2_after_ctx1_drop")
        .unwrap();

    let output = receiver.stop_and_get_output();
    assert_contains_all(
        &output,
        &[
            "MULT CTX1 log info V 1 [Context1_message]",
            "MULT CTX2 log info V 1 [Context2_message]",
            "MULT CTX2 log info V 1 [Context2_after_ctx1_drop]",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_clone_application_handle() {
    // Test that cloning the application handle works correctly
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"CLON").unwrap();
    let ctx_id = DltId::new(b"CTX1").unwrap();

    let receiver = DltReceiver::start();

    let app = DltApplication::register(&app_id, "Clone test app").unwrap();
    let app_clone = app.clone();

    // Create context from original
    let ctx1 = app.create_context(&ctx_id, "Context 1").unwrap();

    // Drop original app
    drop(app);

    // Create context from clone - should still work
    let ctx2_id = DltId::new(b"CTX2").unwrap();
    let ctx2 = app_clone.create_context(&ctx2_id, "Context 2").unwrap();

    // Both contexts should work
    ctx1.log(DltLogLevel::Info, "From_original_app").unwrap();
    ctx2.log(DltLogLevel::Info, "From_cloned_app").unwrap();

    let output = receiver.stop_and_get_output();
    assert_contains_all(
        &output,
        &[
            "CLON CTX1 log info V 1 [From_original_app]",
            "CLON CTX2 log info V 1 [From_cloned_app]",
        ],
    );
}

#[tokio::test]
#[serial]
async fn test_f64_write() {
    ensure_dlt_daemon_running();

    let app_id = DltId::new(b"F64").unwrap();
    let ctx_id = DltId::new(b"CTX1").unwrap();

    let receiver = DltReceiver::start();

    let app = DltApplication::register(&app_id, "f64 write test").unwrap();
    let ctx1 = app.create_context(&ctx_id, "Context 1").unwrap();
    let mut log_writer = ctx1
        .log_write_start(DltLogLevel::Info)
        .expect("Failed to start log");
    log_writer.write_float64(42.42_f64).unwrap();
    log_writer.finish().unwrap();

    let output = receiver.stop_and_get_output();
    assert_contains_all(&output, &["F64- CTX1 log info V 1 [42.42]"]);
}
