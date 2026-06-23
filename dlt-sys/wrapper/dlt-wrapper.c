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

// Requires system DLT headers - set DLT_INCLUDE_DIR environment variable or modify include paths accordingly.
#include "dlt-wrapper.h"
#include <dlt/dlt_user.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>

DltReturnValue registerApplication(const char *appId, const char *appDescription) {
    if (appId == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }
    return dlt_register_app(appId, appDescription);
}

DltReturnValue unregisterApplicationFlushBufferedLogs(void) {
    typedef DltReturnValue (*dlt_unregister_app_flush_buffered_logs_fn)(void);
    static dlt_unregister_app_flush_buffered_logs_fn flush_fn = NULL;
    static int flush_fn_initialized = 0;

    if (!flush_fn_initialized) {
        flush_fn = (dlt_unregister_app_flush_buffered_logs_fn)dlsym(
            RTLD_DEFAULT,
            "dlt_unregister_app_flush_buffered_logs"
        );
        flush_fn_initialized = 1;
    }

    if (flush_fn != NULL) {
        return flush_fn();
    }

    // Fallback for older libdlt releases without buffered-flush API.
    return dlt_unregister_app();
}

DltReturnValue dltFree(void) {
    return dlt_free();
}

DltContext *createContext(void) {
    return (DltContext *)calloc(1, sizeof(DltContext));
}

void freeContext(DltContext *context) {
    if (context != NULL) {
        free(context);
    }
}

DltReturnValue registerContext(const char *contextId, const char *contextDescription, DltContext* context) {
    if (contextId == NULL) {
        return DLT_RETURN_ERROR;
    }

    return dlt_register_context(context, contextId, contextDescription);
}

DltReturnValue unregisterContext(DltContext *context) {
    if (context == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_unregister_context(context);
}

DltReturnValue getContextId(DltContext *context, char context_id[DLT_ID_SIZE]) {
    if (context == NULL || context_id == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    memcpy(context_id, context->contextID, DLT_ID_SIZE);
    return DLT_RETURN_OK;
}

DltReturnValue getContextLogLevel(DltContext *context, int32_t *log_level) {
    if (context == NULL || log_level == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    if (context->log_level_ptr == NULL) {
        *log_level = DLT_LOG_DEFAULT;
        return DLT_RETURN_OK;
    }

    *log_level = (int32_t)(*context->log_level_ptr);
    return DLT_RETURN_OK;
}

DltReturnValue getContextTraceStatus(DltContext *context, int32_t *trace_status) {
    if (context == NULL || trace_status == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    if (context->trace_status_ptr == NULL) {
        *trace_status = DLT_TRACE_STATUS_DEFAULT;
        return DLT_RETURN_OK;
    }

    *trace_status = (int32_t)(*context->trace_status_ptr);
    return DLT_RETURN_OK;
}

DltReturnValue logDlt(DltContext *context, DltLogLevelType logLevel, const char *message) {
    if (context == NULL || message == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    DLT_LOG(*context, logLevel, DLT_CSTRING(message));
    return DLT_RETURN_OK;
}

DltReturnValue logDltString(DltContext *context, DltLogLevelType logLevel, const char *message) {
    return logDlt(context, logLevel, message);
}

DltReturnValue logDltUint(DltContext *context, DltLogLevelType logLevel, uint32_t value) {
    if (context == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    DLT_LOG(*context, logLevel, DLT_UINT32(value));
    return DLT_RETURN_OK;
}

DltReturnValue logDltInt(DltContext *context, DltLogLevelType logLevel, int32_t value) {
    if (context == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    DLT_LOG(*context, logLevel, DLT_INT32(value));
    return DLT_RETURN_OK;
}

DltContextData *createContextData(void) {
    return (DltContextData *)calloc(1, sizeof(DltContextData));
}

void freeContextData(DltContextData *log) {
    if (log != NULL) {
        free(log);
    }
}

DltReturnValue dltUserLogWriteStart(DltContext *context, DltContextData *log, DltLogLevelType logLevel) {
    if (context == NULL || log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_start(context, log, logLevel);
}

DltReturnValue dltUserLogWriteFinish(DltContextData *log) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_finish(log);
}

DltReturnValue setContextDataUserTimestamp(DltContextData *log, uint32_t timestamp) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    log->use_timestamp = DLT_USER_TIMESTAMP;
    log->user_timestamp = timestamp;
    return DLT_RETURN_OK;
}

DltReturnValue dltUserLogWriteString(DltContextData *log, const char *text) {
    if (log == NULL || text == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_string(log, text);
}

DltReturnValue dltUserLogWriteUint(DltContextData *log, uint32_t data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_uint(log, data);
}

DltReturnValue dltUserLogWriteInt(DltContextData *log, int32_t data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_int(log, data);
}

DltReturnValue dltUserLogWriteUint64(DltContextData *log, uint64_t data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_uint64(log, data);
}

DltReturnValue dltUserLogWriteInt64(DltContextData *log, int64_t data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_int64(log, data);
}

DltReturnValue dltUserLogWriteFloat32(DltContextData *log, float data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_float32(log, data);
}

DltReturnValue dltUserLogWriteFloat64(DltContextData *log, double data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_float64(log, data);
}

DltReturnValue dltUserLogWriteBool(DltContextData *log, uint8_t data) {
    if (log == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_user_log_write_bool(log, data);
}

DltReturnValue registerLogLevelChangedCallback(
    DltContext *handle,
    void (*callback)(char context_id[DLT_ID_SIZE], uint8_t log_level, uint8_t trace_status)
) {
    typedef DltReturnValue (*dlt_register_log_level_changed_callback_fn)(
        DltContext *,
        void (*)(char context_id[DLT_ID_SIZE], uint8_t log_level, uint8_t trace_status)
    );
    static dlt_register_log_level_changed_callback_fn callback_fn = NULL;
    static int callback_fn_initialized = 0;

    if (handle == NULL || callback == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    if (!callback_fn_initialized) {
        callback_fn = (dlt_register_log_level_changed_callback_fn)dlsym(
            RTLD_DEFAULT,
            "dlt_register_log_level_changed_callback"
        );
        callback_fn_initialized = 1;
    }

    if (callback_fn == NULL) {
        // Optional API not present in older libdlt releases.
        return DLT_RETURN_ERROR;
    }

    return callback_fn(handle, callback);
}
