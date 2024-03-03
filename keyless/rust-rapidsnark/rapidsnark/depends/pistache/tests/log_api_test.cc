/*
 * SPDX-FileCopyrightText: 2020 Michael Ellison
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <utility>
#include <vector>

#include <pistache/log.h>

#include <gtest/gtest.h>

using namespace Pistache;

// Test that the PISTACHE_LOG_STRING_* macros guard against accessing a null logger.
TEST(logger_test, macros_guard_null_logger)
{
    PISTACHE_STRING_LOGGER_T logger = PISTACHE_NULL_STRING_LOGGER;

    PISTACHE_LOG_STRING_FATAL(logger, "test_message_1_fatal");
    PISTACHE_LOG_STRING_ERROR(logger, "test_message_2_error");
    PISTACHE_LOG_STRING_WARN(logger, "test_message_3_warn");
    PISTACHE_LOG_STRING_INFO(logger, "test_message_4_info");
    PISTACHE_LOG_STRING_DEBUG(logger, "test_message_5_debug");
    PISTACHE_LOG_STRING_TRACE(logger, "test_message_6_trace");

    // Expect no death from accessing the null logger.
}

// Test that the PISTACHE_LOG_STRING_* macros access a default logger.
TEST(logger_test, macros_access_default_logger)
{
    PISTACHE_STRING_LOGGER_T logger = PISTACHE_DEFAULT_STRING_LOGGER;

    PISTACHE_LOG_STRING_FATAL(logger, "test_message_1_fatal");
    PISTACHE_LOG_STRING_ERROR(logger, "test_message_2_error");
    PISTACHE_LOG_STRING_WARN(logger, "test_message_3_warn");
    PISTACHE_LOG_STRING_INFO(logger, "test_message_4_info");
    PISTACHE_LOG_STRING_DEBUG(logger, "test_message_5_debug");
    PISTACHE_LOG_STRING_TRACE(logger, "test_message_6_trace");

    // Expect no death from using the default handler. The only output of the
    // default logger is to stdout, so output cannot be confirmed by gtest.
}
