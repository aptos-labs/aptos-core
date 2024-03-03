/*
 * SPDX-FileCopyrightText: 2021 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <cstdint>

namespace Pistache::Meta::Hash
{
    static constexpr uint64_t val64   = 0xcbf29ce484222325;
    static constexpr uint64_t prime64 = 0x100000001b3;

    inline constexpr uint64_t fnv1a(const char* const str, const uint64_t value = val64) noexcept
    {
        return (str[0] == '\0') ? value : fnv1a(&str[1], (value ^ uint64_t(str[0])) * prime64);
    }
} // namespace Pistache::Meta::Hash
