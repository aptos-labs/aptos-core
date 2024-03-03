/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <stdexcept>

namespace Pistache::Tcp
{

    class SocketError : public std::runtime_error
    {
    public:
        explicit SocketError(const char* what_arg)
            : std::runtime_error(what_arg)
        { }
    };

    class ServerError : public std::runtime_error
    {
    public:
        explicit ServerError(const char* what_arg)
            : std::runtime_error(what_arg)
        { }
    };

} // namespace Pistache::Tcp
