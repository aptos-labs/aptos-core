/*
 * SPDX-FileCopyrightText: 2019 mohsenomidi
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/async.h>
#include <pistache/client.h>
#include <pistache/common.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>

#include <gtest/gtest.h>

#include <chrono>
#include <fstream>
#include <future>
#include <string>

using namespace Pistache;

struct HelloHandlerWithDelay : public Http::Handler
{
    HTTP_PROTOTYPE(HelloHandlerWithDelay)

    explicit HelloHandlerWithDelay(int delay = 0)
        : delay_(delay)
    { }

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        std::this_thread::sleep_for(std::chrono::seconds(delay_));
        writer.send(Http::Code::Ok, "Hello, World!");
    }

    int delay_;
};

int clientLogicFunc(int response_size, const std::string& server_page,
                    int wait_seconds)
{
    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    auto rb              = client.get(server_page);
    int resolver_counter = 0;
    int reject_counter   = 0;
    for (int i = 0; i < response_size; ++i)
    {
        auto response = rb.send();
        response.then(
            [&resolver_counter](Http::Response resp) {
                std::cout << "Response code is " << resp.code() << std::endl;
                if (resp.code() == Http::Code::Ok)
                {
                    ++resolver_counter;
                }
            },
            [&reject_counter](std::exception_ptr exc) {
                PrintException excPrinter;
                std::cout << "Reject with reason: ";
                excPrinter(exc);
                ++reject_counter;
            });
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);
    barrier.wait_for(std::chrono::seconds(wait_seconds));

    client.shutdown();

    std::cout << "resolves: " << resolver_counter
              << ", rejects: " << reject_counter << "\n";

    return resolver_counter;
}

TEST(
    http_server_test,
    multiple_client_with_requests_to_multithreaded_server_threadName_null_str)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    const std::string threadName_null_str = "";

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(2).threadsName(
        threadName_null_str);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandlerWithDelay>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const int CLIENT_REQUEST_SIZE = 2;
    const int SIX_SECONDS_TIMOUT  = 6;
    std::future<int> result(std::async(clientLogicFunc, CLIENT_REQUEST_SIZE,
                                       server_address, SIX_SECONDS_TIMOUT));

    int res1 = result.get();

    server.shutdown();

    ASSERT_EQ(res1, CLIENT_REQUEST_SIZE);
}

TEST(
    http_server_test,
    multiple_client_with_requests_to_multithreaded_server_threadName_single_char)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    const std::string threadName_single_char = "a";

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(2).threadsName(
        threadName_single_char);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandlerWithDelay>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const int CLIENT_REQUEST_SIZE = 2;
    const int SIX_SECONDS_TIMOUT  = 6;
    std::future<int> result(std::async(clientLogicFunc, CLIENT_REQUEST_SIZE,
                                       server_address, SIX_SECONDS_TIMOUT));

    int res1 = result.get();

    server.shutdown();

    ASSERT_EQ(res1, CLIENT_REQUEST_SIZE);
}

TEST(
    http_server_test,
    multiple_client_with_requests_to_multithreaded_server_threadName_max_length)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    const std::string threadName_max_length = "0123456789abcdef";

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(2).threadsName(
        threadName_max_length);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandlerWithDelay>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const int CLIENT_REQUEST_SIZE = 2;
    const int SIX_SECONDS_TIMOUT  = 6;
    std::future<int> result(std::async(clientLogicFunc, CLIENT_REQUEST_SIZE,
                                       server_address, SIX_SECONDS_TIMOUT));

    int res1 = result.get();

    server.shutdown();

    ASSERT_EQ(res1, CLIENT_REQUEST_SIZE);
}

TEST(
    http_server_test,
    multiple_client_with_requests_to_multithreaded_server_threadName_exceed_length)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    const std::string threadName_exceed_length = "0123456789abcdefghi";

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(2).threadsName(
        threadName_exceed_length);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandlerWithDelay>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const int CLIENT_REQUEST_SIZE = 2;
    const int SIX_SECONDS_TIMOUT  = 6;
    std::future<int> result(std::async(clientLogicFunc, CLIENT_REQUEST_SIZE,
                                       server_address, SIX_SECONDS_TIMOUT));

    int res1 = result.get();

    server.shutdown();

    ASSERT_EQ(res1, CLIENT_REQUEST_SIZE);
}
