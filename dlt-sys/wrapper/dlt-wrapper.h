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

#ifndef DLT_WRAPPER_H
#define DLT_WRAPPER_H

// Requires system DLT headers - set DLT_INCLUDE_DIR environment variable
#include <dlt/dlt.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Application management
DltReturnValue registerApplication(const char *appId, const char *appDescription);
DltReturnValue unregisterApplicationFlushBufferedLogs(void);
DltReturnValue dltFree(void);

// Context management
DltContext *registerContext(const char *contextId, const char *contextDescription);
DltReturnValue unregisterContext(DltContext *context);

// Logging functions
DltReturnValue logDlt(DltContext *context, DltLogLevelType logLevel, const char *message);
DltReturnValue logDltString(DltContext *context, DltLogLevelType logLevel, const char *message);
DltReturnValue logDltUint(DltContext *context, DltLogLevelType logLevel, uint32_t value);
DltReturnValue logDltInt(DltContext *context, DltLogLevelType logLevel, int32_t value);

// Log write API (for structured logging)
DltReturnValue dltUserLogWriteStart(DltContext *context, DltContextData *log, DltLogLevelType logLevel);
DltReturnValue dltUserLogWriteFinish(DltContextData *log);
DltReturnValue dltUserLogWriteString(DltContextData *log, const char *text);
DltReturnValue dltUserLogWriteUint(DltContextData *log, uint32_t data);
DltReturnValue dltUserLogWriteInt(DltContextData *log, int32_t data);
DltReturnValue dltUserLogWriteUint64(DltContextData *log, uint64_t data);
DltReturnValue dltUserLogWriteInt64(DltContextData *log, int64_t data);
DltReturnValue dltUserLogWriteFloat32(DltContextData *log, float data);
DltReturnValue dltUserLogWriteFloat64(DltContextData *log, double data);
DltReturnValue dltUserLogWriteBool(DltContextData *log, uint8_t data);

// Callback for log level changes
DltReturnValue registerLogLevelChangedCallback(
    DltContext *handle,
    void (*callback)(char context_id[DLT_ID_SIZE], uint8_t log_level, uint8_t trace_status)
);

#ifdef __cplusplus
}
#endif

#endif // DLT_WRAPPER_H
