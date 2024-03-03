/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* timer_pool.cc
   Mathieu Stefani, 09 février 2016

   Implementation of the timer pool
*/

#include <pistache/os.h>
#include <pistache/timer_pool.h>

#include <cassert>

#include <sys/timerfd.h>

namespace Pistache
{

    TimerPool::Entry::Entry()
        : fd_(-1)
        , registered(false)
    {
        state.store(static_cast<uint32_t>(State::Idle));
    }

    TimerPool::Entry::~Entry()
    {
        if (fd_ != -1)
            close(fd_);
    }

    Fd TimerPool::Entry::fd() const
    {
        assert(fd_ != -1);

        return fd_;
    }

    void TimerPool::Entry::initialize()
    {
        if (fd_ == -1)
        {
            fd_ = TRY_RET(timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK));
        }
    }

    void TimerPool::Entry::disarm()
    {
        assert(fd_ != -1);

        itimerspec spec;
        spec.it_interval.tv_sec  = 0;
        spec.it_interval.tv_nsec = 0;

        spec.it_value.tv_sec  = 0;
        spec.it_value.tv_nsec = 0;

        TRY(timerfd_settime(fd_, 0, &spec, nullptr));
    }

    void TimerPool::Entry::registerReactor(const Aio::Reactor::Key& key,
                                           Aio::Reactor* reactor)
    {
        if (!registered)
        {
            reactor->registerFd(key, fd_, Polling::NotifyOn::Read);
            registered = true;
        }
    }

    void TimerPool::Entry::armMs(std::chrono::milliseconds value)
    {
        itimerspec spec;
        spec.it_interval.tv_sec  = 0;
        spec.it_interval.tv_nsec = 0;

        if (value.count() < 1000)
        {
            spec.it_value.tv_sec  = 0;
            spec.it_value.tv_nsec = std::chrono::duration_cast<std::chrono::nanoseconds>(value).count();
        }
        else
        {
            spec.it_value.tv_sec  = std::chrono::duration_cast<std::chrono::seconds>(value).count();
            spec.it_value.tv_nsec = 0;
        }
        TRY(timerfd_settime(fd_, 0, &spec, nullptr));
    }

    TimerPool::TimerPool(size_t initialSize)
    {
        for (size_t i = 0; i < initialSize; ++i)
        {
            timers.push_back(std::make_shared<TimerPool::Entry>());
        }
    }

    std::shared_ptr<TimerPool::Entry> TimerPool::pickTimer()
    {
        for (auto& entry : timers)
        {
            auto curState = static_cast<uint32_t>(TimerPool::Entry::State::Idle);
            auto newState = static_cast<uint32_t>(TimerPool::Entry::State::Used);
            if (entry->state.compare_exchange_strong(curState, newState))
            {
                entry->initialize();
                return entry;
            }
        }

        return nullptr;
    }

    void TimerPool::releaseTimer(const std::shared_ptr<Entry>& timer)
    {
        timer->state.store(static_cast<uint32_t>(TimerPool::Entry::State::Idle));
    }

} // namespace Pistache
