/*
 * SPDX-FileCopyrightText: 2019 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/mailbox.h>
#include <pistache/reactor.h>

#include <gtest/gtest.h>

#include <chrono>
#include <iostream>
#include <memory>
#include <thread>
#include <unordered_set>

using namespace Pistache;

class TransportMock : public Aio::Handler
{
    PROTOTYPE_OF(Aio::Handler, TransportMock)

public:
    TransportMock()
        : queue_()
    { }

    TransportMock(const TransportMock&)
        : queue_()
    { }

    void onReady(const Aio::FdSet& fds) override
    {
        for (const auto& entry : fds)
        {
            if (entry.getTag() == queue_.tag())
            {
                while (true)
                {
                    auto value = queue_.popSafe();
                    if (!value)
                        break;
                    values_.insert(*value);
                }
            }
        }
    }

    void registerPoller(Polling::Epoll& poller) override { queue_.bind(poller); }

    void push(int value) { queue_.push(value); }

    const std::unordered_set<int>& values() { return values_; }

private:
    PollableQueue<int> queue_;
    std::unordered_set<int> values_;
};

TEST(reactor_test, reactor_creation)
{
    constexpr size_t NUM_THREADS          = 2;
    std::shared_ptr<Aio::Reactor> reactor = Aio::Reactor::create();
    reactor->init(Aio::AsyncContext(NUM_THREADS));
    auto key = reactor->addHandler(std::make_shared<TransportMock>());
    reactor->run();

    auto handlers = reactor->handlers(key);

    const size_t NUM_VALUES                   = 4;
    const int values[NUM_THREADS][NUM_VALUES] = { { 1, 2, 3, 4 }, { 5, 6, 7, 8 } };

    for (size_t i = 0; i < handlers.size(); ++i)
    {
        auto transport = std::static_pointer_cast<TransportMock>(handlers[i]);
        for (size_t j = 0; j < NUM_VALUES; ++j)
        {
            transport->push(values[i][j]);
        }
    }

    std::this_thread::sleep_for(std::chrono::seconds(1));

    reactor->shutdown();

    ASSERT_EQ(handlers.size(), NUM_THREADS);

    for (size_t i = 0; i < handlers.size(); ++i)
    {
        auto transport              = std::static_pointer_cast<TransportMock>(handlers[i]);
        const auto& resulted_values = transport->values();
        for (size_t j = 0; j < NUM_VALUES; ++j)
        {
            ASSERT_NE(resulted_values.find(values[i][j]), resulted_values.end());
        }
    }
}

TEST(reactor_test, reactor_exceed_max_threads)
{
    constexpr size_t MAX_SUPPORTED_THREADS = 255;
    std::shared_ptr<Aio::Reactor> reactor  = Aio::Reactor::create();
    ASSERT_THROW(reactor->init(Aio::AsyncContext(5 * MAX_SUPPORTED_THREADS + 1)),
                 std::runtime_error);
}
