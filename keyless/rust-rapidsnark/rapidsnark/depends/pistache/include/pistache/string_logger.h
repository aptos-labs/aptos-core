/*
 * SPDX-FileCopyrightText: 2020 Michael Ellison
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* log.h
   Michael Ellison, 27 May 2020

   String logger definitions - to be used via the macros defined in log.h, or
   passed into a Pistache library function as a logging endpoint.
*/

#pragma once

#include <iostream>
#include <memory>

namespace Pistache::Log
{

    enum class Level {
        TRACE,
        DEBUG,
        INFO,
        WARN,
        ERROR,
        FATAL
    };

    class StringLogger
    {
    public:
        virtual void log(Level level, const std::string& message) = 0;
        virtual bool isEnabledFor(Level level) const              = 0;

        virtual ~StringLogger() = default;
    };

    class StringToStreamLogger : public StringLogger
    {
    public:
        explicit StringToStreamLogger(Level level, std::ostream* out = &std::cerr)
            : level_(level)
            , out_(out)
        { }
        ~StringToStreamLogger() override = default;

        void log(Level level, const std::string& message) override;
        bool isEnabledFor(Level level) const override;

    private:
        Level level_;
        std::ostream* out_;
    };

} // namespace Pistache::Log
