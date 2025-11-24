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
#include <stdlib.h>
#include <string.h>

DltReturnValue registerApplication(const char *appId, const char *appDescription) {
    if (appId == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }
    return dlt_register_app(appId, appDescription);
}

DltReturnValue unregisterApplicationFlushBufferedLogs(void) {
    return dlt_unregister_app_flush_buffered_logs();
}

DltReturnValue dltFree(void) {
    return dlt_free();
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
    if (handle == NULL || callback == NULL) {
        return DLT_RETURN_WRONG_PARAMETER;
    }

    return dlt_register_log_level_changed_callback(handle, callback);
}
