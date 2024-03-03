/*
 * SPDX-FileCopyrightText: 2019 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <cstddef>
#include <cstdint>
#include <limits>

#include <chrono>

// Allow compile-time overload
namespace Pistache::Const
{
    static constexpr size_t MaxBacklog     = 128;
    static constexpr size_t MaxEvents      = 1024;
    static constexpr size_t MaxBuffer      = 4096;
    static constexpr size_t DefaultWorkers = 1;

    static constexpr size_t DefaultTimerPoolSize = 128;

    // Defined from CMakeLists.txt in project root
    static constexpr size_t DefaultMaxRequestSize    = 4096;
    static constexpr size_t DefaultMaxResponseSize   = std::numeric_limits<uint32_t>::max();
    static constexpr auto DefaultHeaderTimeout       = std::chrono::seconds(60);
    static constexpr auto DefaultBodyTimeout         = std::chrono::seconds(60);
    static constexpr auto DefaultKeepaliveTimeout    = std::chrono::seconds(300);
    static constexpr auto DefaultSSLHandshakeTimeout = std::chrono::seconds(10);
    static constexpr size_t ChunkSize                = 1024;

    static constexpr uint16_t HTTP_STANDARD_PORT = 80;
} // namespace Pistache::Const
