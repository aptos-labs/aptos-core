/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* timer_pool.h
   Mathieu Stefani, 09 février 2016

   A pool of timer fd to avoid creating fds everytime we need a timer and
   thus reduce the total number of system calls.

   Most operations are lock-free except resize operations needed when the
   pool is empty, in which case it's blocking but we expect it to be rare.
*/

#pragma once

#include <pistache/config.h>
#include <pistache/os.h>
#include <pistache/reactor.h>

#include <atomic>
#include <memory>
#include <vector>

#include <cassert>
#include <unistd.h>

namespace Pistache
{

    class TimerPool
    {
    public:
        explicit TimerPool(size_t initialSize = Const::DefaultTimerPoolSize);

        struct Entry
        {

            friend class TimerPool;

            Entry();
            ~Entry();

            Fd fd() const;

            void initialize();

            template <typename Duration>
            void arm(Duration duration)
            {
                assert(fd_ != -1 && "Entry is not initialized");

                armMs(std::chrono::duration_cast<std::chrono::milliseconds>(duration));
            }

            void disarm();

            void registerReactor(const Aio::Reactor::Key& key, Aio::Reactor* reactor);

        private:
            void armMs(std::chrono::milliseconds value);
            enum class State : uint32_t { Idle,
                                          Used };
            std::atomic<uint32_t> state;
            Fd fd_;
            bool registered;
        };

        std::shared_ptr<Entry> pickTimer();
        static void releaseTimer(const std::shared_ptr<Entry>& timer);

    private:
        std::vector<std::shared_ptr<Entry>> timers;
    };

} // namespace Pistache
