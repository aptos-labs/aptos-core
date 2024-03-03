/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* os.h
   Mathieu Stefani, 13 August 2015

   Operating system specific functions
*/

#pragma once

#include <pistache/common.h>
#include <pistache/config.h>
#include <pistache/flags.h>

#include <bitset>
#include <chrono>
#include <memory>
#include <vector>

#include <sched.h>

namespace Pistache
{

    using Fd = int;

    uint hardware_concurrency();
    bool make_non_blocking(int fd);

    class CpuSet
    {
    public:
        static constexpr size_t Size = 1024;

        CpuSet();
        explicit CpuSet(std::initializer_list<size_t> cpus);

        void clear();
        CpuSet& set(size_t cpu);
        CpuSet& unset(size_t cpu);

        CpuSet& set(std::initializer_list<size_t> cpus);
        CpuSet& unset(std::initializer_list<size_t> cpus);

        CpuSet& setRange(size_t begin, size_t end);
        CpuSet& unsetRange(size_t begin, size_t end);

        bool isSet(size_t cpu) const;
        size_t count() const;

        cpu_set_t toPosix() const;

    private:
        std::bitset<Size> bits;
    };

    namespace Polling
    {

        enum class Mode { Level,
                          Edge };

        enum class NotifyOn {
            None = 0,

            Read     = 1,
            Write    = Read << 1,
            Hangup   = Read << 2,
            Shutdown = Read << 3
        };

        DECLARE_FLAGS_OPERATORS(NotifyOn)

        struct Tag
        {
            friend class Epoll;

            explicit constexpr Tag(uint64_t value)
                : value_(value)
            { }

            constexpr uint64_t value() const { return value_; }

            friend constexpr bool operator==(Tag lhs, Tag rhs);

        private:
            uint64_t value_;
        };

        inline constexpr bool operator==(Tag lhs, Tag rhs)
        {
            return lhs.value_ == rhs.value_;
        }

        struct Event
        {
            explicit Event(Tag _tag);

            Flags<NotifyOn> flags;
            Tag tag;
        };

        class Epoll
        {
        public:
            Epoll();
            ~Epoll();

            void addFd(Fd fd, Flags<NotifyOn> interest, Tag tag, Mode mode = Mode::Level);
            void addFdOneShot(Fd fd, Flags<NotifyOn> interest, Tag tag,
                              Mode mode = Mode::Level);

            void removeFd(Fd fd);
            void rearmFd(Fd fd, Flags<NotifyOn> interest, Tag tag,
                         Mode mode = Mode::Level);

            int poll(std::vector<Event>& events, const std::chrono::milliseconds timeout = std::chrono::milliseconds(-1)) const;

        private:
            static int toEpollEvents(const Flags<NotifyOn>& interest);
            static Flags<NotifyOn> toNotifyOn(int events);
            Fd epoll_fd;
        };

    } // namespace Polling

    class NotifyFd
    {
    public:
        NotifyFd();

        Polling::Tag bind(Polling::Epoll& poller);

        bool isBound() const;

        Polling::Tag tag() const;

        void notify() const;

        void read() const;
        bool tryRead() const;

    private:
        Fd event_fd;
    };

} // namespace Pistache
