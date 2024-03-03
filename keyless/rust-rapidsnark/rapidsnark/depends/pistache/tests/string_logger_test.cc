/*
 * SPDX-FileCopyrightText: 2020 Michael Ellison
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <utility>
#include <vector>

#include <pistache/string_logger.h>

#include <gtest/gtest.h>

using namespace Pistache;

TEST(logger_test, logger_guards_by_level)
{
    std::stringstream ss;
    std::shared_ptr<::Pistache::Log::StringLogger> logger = std::make_shared<::Pistache::Log::StringToStreamLogger>(::Pistache::Log::Level::WARN, &ss);

    logger->log(::Pistache::Log::Level::FATAL, "test_message_1_fatal");
    logger->log(::Pistache::Log::Level::ERROR, "test_message_2_error");
    logger->log(::Pistache::Log::Level::WARN, "test_message_3_warn");
    logger->log(::Pistache::Log::Level::INFO, "test_message_4_info");
    logger->log(::Pistache::Log::Level::DEBUG, "test_message_5_debug");
    logger->log(::Pistache::Log::Level::TRACE, "test_message_6_trace");
    logger->log(::Pistache::Log::Level::ERROR, "test_message_7_error");
    logger->log(::Pistache::Log::Level::DEBUG, "test_message_8_debug");
    logger->log(::Pistache::Log::Level::FATAL, "test_message_9_fatal");

    std::string expected_string = "test_message_1_fatal\n"
                                  "test_message_2_error\n"
                                  "test_message_3_warn\n"
                                  "test_message_7_error\n"
                                  "test_message_9_fatal\n";

    ASSERT_EQ(ss.str(), expected_string);
}
