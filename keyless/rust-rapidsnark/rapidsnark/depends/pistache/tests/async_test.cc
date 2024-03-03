/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>

#include <pistache/async.h>
#include <pistache/common.h>

#include <algorithm>
#include <condition_variable>
#include <deque>
#include <mutex>
#include <random>
#include <thread>

using namespace Pistache;

Async::Promise<int> doAsync(int N)
{
    Async::Promise<int> promise(
        [=](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            std::thread thr(
                [=](Async::Resolver resolve) mutable {
                    std::this_thread::sleep_for(std::chrono::seconds(1));
                    resolve(N * 2);
                },
                std::move(resolve));

            thr.detach();
        });

    return promise;
}

template <typename T, typename Func>
Async::Promise<T> doAsyncTimed(std::chrono::seconds time, T val, Func func)
{
    Async::Promise<T> promise(
        [=](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            std::thread thr(
                [=](Async::Resolver resolve) mutable {
                    std::this_thread::sleep_for(time);
                    resolve(func(val));
                },
                std::move(resolve));

            thr.detach();
        });

    return promise;
}

TEST(async_test, basic_test)
{
    Async::Promise<int> p1([](Async::Resolver& resolv, Async::Rejection& /*reject*/) {
        resolv(10);
    });

    ASSERT_TRUE(p1.isFulfilled());

    int val { 0 };
    p1.then([&](int v) { val = v; }, Async::NoExcept);
    ASSERT_EQ(val, 10);

    {
        Async::Promise<int> p2 = doAsync(10);
        p2.then([](int result) { ASSERT_EQ(result, 20); }, Async::NoExcept);
    }

    std::this_thread::sleep_for(std::chrono::seconds(2));

    Async::Promise<int> p3([](Async::Resolver& /*resolv*/, Async::Rejection& reject) {
        reject(std::runtime_error("Because I decided"));
    });

    ASSERT_TRUE(p3.isRejected());
    p3.then([](int) { ASSERT_TRUE(false); },
            [](std::exception_ptr eptr) {
                ASSERT_THROW(std::rethrow_exception(eptr), std::runtime_error);
            });

    auto p4 = Async::Promise<int>::resolved(10);
    ASSERT_TRUE(p4.isFulfilled());

    auto p5 = Async::Promise<void>::resolved();
    ASSERT_TRUE(p5.isFulfilled());

    auto p6 = Async::Promise<int>::rejected(std::invalid_argument("Invalid"));
    ASSERT_TRUE(p6.isRejected());
}

TEST(async_test, error_test)
{
    Async::Promise<int> p1(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            ASSERT_THROW(resolve(10.5), Async::BadType);
        });
}

TEST(async_test, void_promise)
{
    Async::Promise<void> p1(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            resolve();
        });

    ASSERT_TRUE(p1.isFulfilled());

    bool thenCalled { false };
    p1.then([&]() { thenCalled = true; }, Async::NoExcept);

    ASSERT_TRUE(thenCalled);

    Async::Promise<int> p2(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            ASSERT_THROW(resolve(), Async::Error);
        });

    Async::Promise<void> p3(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            ASSERT_THROW(resolve(10), Async::Error);
        });
}

TEST(async_test, chain_test)
{
    Async::Promise<int> p1(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            resolve(10);
        });

    p1.then([](int result) { return result * 2; }, Async::NoExcept)
        .then([](int result) { std::cout << "Result = " << result << std::endl; },
              Async::NoExcept);

    Async::Promise<int> p2(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            resolve(10);
        });

    p2.then([](int result) { return result * 2.2901; }, Async::IgnoreException)
        .then(
            [](double result) {
                std::cout << "Result = " << result << std::endl;
            },
            Async::IgnoreException);

    enum class Test { Foo,
                      Bar };

    Async::Promise<Test> p3(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            resolve(Test::Foo);
        });

    p3.then(
          [](Test result) {
              return Async::Promise<std::string>(
                  [=](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
                      switch (result)
                      {
                      case Test::Foo:
                          resolve(std::string("Foo"));
                          break;
                      case Test::Bar:
                          resolve(std::string("Bar"));
                      }
                  });
          },
          Async::NoExcept)
        .then([](std::string str) { ASSERT_EQ(str, "Foo"); }, Async::NoExcept);

    Async::Promise<Test> p4(
        [](Async::Resolver& resolve, Async::Rejection& /*reject*/) {
            resolve(Test::Bar);
        });

    p4.then(
          [](Test result) {
              return Async::Promise<std::string>(
                  [=](Async::Resolver& resolve, Async::Rejection& reject) {
                      switch (result)
                      {
                      case Test::Foo:
                          resolve(std::string("Foo"));
                          break;
                      case Test::Bar:
                          reject(std::runtime_error("Invalid"));
                      }
                  });
          },
          Async::NoExcept)
        .then(
            [](std::string /*str*/) {
                ASSERT_TRUE(false);
            },
            [](std::exception_ptr exc) {
                ASSERT_THROW(std::rethrow_exception(exc), std::runtime_error);
            });

    auto p5 = doAsync(10);
    p5.then([](int result) { return result * 3.51; }, Async::NoExcept)
        .then([](double result) { ASSERT_EQ(result, 20 * 3.51); },
              Async::NoExcept);

    auto p6 = doAsync(20);
    p6.then([](int result) { return doAsync(result - 5); }, Async::NoExcept)
        .then([](int result) { ASSERT_EQ(result, 70); }, Async::NoExcept);

    std::this_thread::sleep_for(std::chrono::seconds(2));
}

TEST(async_test, when_all)
{
    auto p1 = Async::Promise<int>::resolved(10);
    int p2  = 123;
    auto p3 = Async::Promise<std::string>::resolved("Hello");
    auto p4 = Async::Promise<void>::resolved();

    bool resolved { false };

    Async::whenAll(p1, p2, p3)
        .then(
            [&](const std::tuple<int, int, std::string>& results) {
                resolved = true;
                ASSERT_EQ(std::get<0>(results), 10);
                ASSERT_EQ(std::get<1>(results), 123);
                ASSERT_EQ(std::get<2>(results), "Hello");
            },
            Async::NoExcept);

    ASSERT_TRUE(resolved);

    std::vector<Async::Promise<int>> vec;
    vec.push_back(std::move(p1));
    vec.push_back(Async::Promise<int>::resolved(p2));

    resolved = false;

    Async::whenAll(std::begin(vec), std::end(vec))
        .then(
            [&](const std::vector<int>& results) {
                resolved = true;
                ASSERT_EQ(results.size(), 2U);
                ASSERT_EQ(results[0], 10);
                ASSERT_EQ(results[1], 123);
            },
            Async::NoExcept);

    ASSERT_TRUE(resolved);

    auto p5 = doAsync(10);
    auto p6 = p5.then([](int result) { return result * 3.1415; }, Async::NoExcept);

    resolved = false;

    Async::whenAll(p5, p6).then(
        [&](std::tuple<int, double> results) {
            ASSERT_EQ(std::get<0>(results), 20);
            ASSERT_EQ(std::get<1>(results), 20 * 3.1415);
            resolved = true;
        },
        Async::NoExcept);

    std::this_thread::sleep_for(std::chrono::seconds(3));
    ASSERT_TRUE(resolved);

    // @Todo: does not compile yet. Figure out why it does not compile with void
    // promises
#if 0
    Async::whenAll(p3, p4).then([](const std::tuple<std::string, void>& results) {
    }, Async::NoExcept);
#endif

    std::vector<Async::Promise<void>> promises;
    promises.push_back(std::move(p4));
    promises.push_back(Async::Promise<void>::resolved());
    auto p7 = Async::whenAll(std::begin(promises), std::end(promises));

    resolved = false;

    p7.then([&]() { resolved = true; }, Async::NoExcept);

    ASSERT_TRUE(resolved);
}

TEST(async_test, when_any)
{
    auto p1 = doAsyncTimed(std::chrono::seconds(2), 10.0,
                           [](double val) { return -val; });
    auto p2 = doAsyncTimed(std::chrono::seconds(1), std::string("Hello"),
                           [](std::string val) {
                               std::transform(std::begin(val), std::end(val),
                                              std::begin(val), ::toupper);
                               return val;
                           });

    bool resolved = false;
    Async::whenAny(p1, p2).then(
        [&](const Async::Any& any) {
            ASSERT_TRUE(any.is<std::string>());

            auto val = any.cast<std::string>();
            ASSERT_EQ(val, "HELLO");

            ASSERT_THROW(any.cast<double>(), Async::BadAnyCast);
            resolved = true;
        },
        Async::NoExcept);

    std::this_thread::sleep_for(std::chrono::seconds(3));
    ASSERT_TRUE(resolved);
}

TEST(async_test, rethrow_test)
{
    auto p1 = Async::Promise<void>(
        [](Async::Resolver& /*resolve*/, Async::Rejection& reject) {
            reject(std::runtime_error("Because"));
        });

    auto p2 = p1.then([]() {}, Async::Throw);

    ASSERT_TRUE(p2.isRejected());
}

template <typename T>
struct MessageQueue
{
public:
    template <typename U>
    void push(U&& arg)
    {
        std::unique_lock<std::mutex> guard(mtx);
        q.push_back(std::forward<U>(arg));
        cv.notify_one();
    }

    T pop()
    {
        std::unique_lock<std::mutex> lock(mtx);
        cv.wait(lock, [=]() { return !q.empty(); });

        T out = std::move(q.front());
        q.pop_front();

        return out;
    }

    bool tryPop(T& out, std::chrono::milliseconds timeout)
    {
        std::unique_lock<std::mutex> lock(mtx);
        if (!cv.wait_for(lock, timeout, [=]() { return !q.empty(); }))
            return false;

        out = std::move(q.front());
        q.pop_front();
        return true;
    }

private:
    std::deque<T> q;
    std::mutex mtx;
    std::condition_variable cv;
};

struct Worker
{
public:
    ~Worker() { thread->join(); }
    void start()
    {
        shutdown.store(false);
        thread.reset(new std::thread([=]() { run(); }));
    }

    void stop() { shutdown.store(true); }

    Async::Promise<int> doWork(int seq)
    {
        return Async::Promise<int>([=](Async::Resolver& resolve,
                                       Async::Rejection& reject) {
            queue.push(new WorkRequest(std::move(resolve), std::move(reject), seq));
        });
    }

private:
    void run()
    {
        while (!shutdown)
        {
            WorkRequest* request;
            if (queue.tryPop(request, std::chrono::milliseconds(200)))
            {
                request->resolve(request->seq);
                delete request;
            }
        }
    }

    struct WorkRequest
    {
        WorkRequest(Async::Resolver resolve, Async::Rejection reject, int seq)
            : resolve(std::move(resolve))
            , reject(std::move(reject))
            , seq(seq)
        { }

        Async::Resolver resolve;
        Async::Rejection reject;
        int seq;
    };

    std::atomic<bool> shutdown;
    MessageQueue<WorkRequest*> queue;

    std::random_device rd;
    std::unique_ptr<std::thread> thread;
};

TEST(async_test, stress_multithreaded_test)
{
    static constexpr size_t OpsPerThread = 100000;
    static constexpr size_t Workers      = 6;
    static constexpr size_t Ops          = OpsPerThread * Workers;

    std::cout << "Starting stress testing promises, hang on, this test might "
                 "take some time to complete"
              << std::endl;
    std::cout << "=================================================" << std::endl;
    std::cout << "Parameters for the test: " << std::endl;
    std::cout << "Workers      -> " << Workers << std::endl;
    std::cout << "OpsPerThread -> " << OpsPerThread << std::endl;
    std::cout << "Total Ops    -> " << Ops << std::endl;
    std::cout << "=================================================" << std::endl;

    std::cout << std::endl
              << std::endl;

    std::vector<std::unique_ptr<Worker>> workers;
    for (size_t i = 0; i < Workers; ++i)
    {
        std::unique_ptr<Worker> wrk(new Worker);
        wrk->start();
        workers.push_back(std::move(wrk));
    }
    std::vector<Async::Promise<int>> promises;
    std::atomic<int> resolved(0);

    size_t wrkIndex = 0;

    for (size_t i = 0; i < Ops; ++i)
    {
        auto& wrk = workers[wrkIndex];
        wrk->doWork(static_cast<int>(i)).then([&](int /*seq*/) {
            ++resolved;
        },
                                              Async::NoExcept);

        wrkIndex = (wrkIndex + 1) % Workers;
    }

    for (;;)
    {
        auto r = resolved.load();
        std::cout << r << " promises resolved" << std::endl;
        if (r == Ops)
            break;
        std::this_thread::sleep_for(std::chrono::milliseconds(500));
    }

    std::cout << "Stopping worker" << std::endl;
    for (auto& wrk : workers)
    {
        wrk->stop();
    }
}

TEST(async_test, chain_rejects)
{
    bool ok = false;
    std::unique_ptr<Async::Rejection> rejecter;
    Async::Promise<int> promise(
        [&](Async::Resolver& /*resolve*/, Async::Rejection& reject) {
            rejecter = std::make_unique<Async::Rejection>(std::move(reject));
        });
    promise.then(
        [](int v) -> Async::Promise<int> {
            return Async::Promise<int>::resolved(v);
        },
        [&](std::exception_ptr e) -> Async::Promise<int> {
            ok = true;
            return Async::Promise<int>::rejected(e);
        });

    ASSERT_FALSE(ok);
    (*rejecter)(std::runtime_error("foo"));
    ASSERT_TRUE(ok);
}
