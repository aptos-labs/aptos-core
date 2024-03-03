/*
 * SPDX-FileCopyrightText: 2020 Michael Ellison
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* log.h
   Michael Ellison, 27 May 2020

   Logging macro definitions - use these to log messages in Pistache.
*/

#pragma once

#include <pistache/string_logger.h>

#ifndef PISTACHE_LOG_STRING_FATAL
#define PISTACHE_LOG_STRING_FATAL(logger, message)                         \
    do                                                                     \
    {                                                                      \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::FATAL)) \
        {                                                                  \
            std::ostringstream oss_;                                       \
            oss_ << message;                                               \
            logger->log(::Pistache::Log::Level::FATAL, oss_.str());        \
        }                                                                  \
    } while (0)
#endif

#ifndef PISTACHE_LOG_STRING_ERROR
#define PISTACHE_LOG_STRING_ERROR(logger, message)                         \
    do                                                                     \
    {                                                                      \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::ERROR)) \
        {                                                                  \
            std::ostringstream oss_;                                       \
            oss_ << message;                                               \
            logger->log(::Pistache::Log::Level::ERROR, oss_.str());        \
        }                                                                  \
    } while (0)
#endif

#ifndef PISTACHE_LOG_STRING_WARN
#define PISTACHE_LOG_STRING_WARN(logger, message)                         \
    do                                                                    \
    {                                                                     \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::WARN)) \
        {                                                                 \
            std::ostringstream oss_;                                      \
            oss_ << message;                                              \
            logger->log(::Pistache::Log::Level::WARN, oss_.str());        \
        }                                                                 \
    } while (0)
#endif

#ifndef PISTACHE_LOG_STRING_INFO
#define PISTACHE_LOG_STRING_INFO(logger, message)                         \
    do                                                                    \
    {                                                                     \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::INFO)) \
        {                                                                 \
            std::ostringstream oss_;                                      \
            oss_ << message;                                              \
            logger->log(::Pistache::Log::Level::INFO, oss_.str());        \
        }                                                                 \
    } while (0)
#endif

#ifndef PISTACHE_LOG_STRING_DEBUG
#define PISTACHE_LOG_STRING_DEBUG(logger, message)                         \
    do                                                                     \
    {                                                                      \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::DEBUG)) \
        {                                                                  \
            std::ostringstream oss_;                                       \
            oss_ << message;                                               \
            logger->log(::Pistache::Log::Level::DEBUG, oss_.str());        \
        }                                                                  \
    } while (0)
#endif

#ifndef PISTACHE_LOG_STRING_TRACE
#ifndef NDEBUG // Only enable trace logging in debug builds.
#define PISTACHE_LOG_STRING_TRACE(logger, message)                         \
    do                                                                     \
    {                                                                      \
        if (logger && logger->isEnabledFor(::Pistache::Log::Level::TRACE)) \
        {                                                                  \
            std::ostringstream oss_;                                       \
            oss_ << message;                                               \
            logger->log(::Pistache::Log::Level::TRACE, oss_.str());        \
        }                                                                  \
    } while (0)
#else
#define PISTACHE_LOG_STRING_TRACE(logger, message)                  \
    do                                                              \
    {                                                               \
        if (0)                                                      \
        {                                                           \
            std::ostringstream oss_;                                \
            oss_ << message;                                        \
            logger->log(::Pistache::Log::Level::TRACE, oss_.str()); \
        }                                                           \
    } while (0)
#endif
#endif

#ifndef PISTACHE_STRING_LOGGER_T
#define PISTACHE_STRING_LOGGER_T \
    std::shared_ptr<::Pistache::Log::StringLogger>
#endif

#ifndef PISTACHE_DEFAULT_STRING_LOGGER
#define PISTACHE_DEFAULT_STRING_LOGGER \
    std::make_shared<::Pistache::Log::StringToStreamLogger>(::Pistache::Log::Level::WARN)
#endif

#ifndef PISTACHE_NULL_STRING_LOGGER
#define PISTACHE_NULL_STRING_LOGGER \
    nullptr
#endif
