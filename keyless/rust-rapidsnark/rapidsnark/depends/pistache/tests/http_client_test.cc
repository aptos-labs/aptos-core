/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/client.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>

#include <gtest/gtest.h>

#include <atomic>
#include <chrono>

using namespace Pistache;

struct HelloHandler : public Http::Handler
{
    HTTP_PROTOTYPE(HelloHandler)

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        writer.send(Http::Code::Ok, "Hello, World!");
    }
};

struct DelayHandler : public Http::Handler
{
    HTTP_PROTOTYPE(DelayHandler)

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        std::this_thread::sleep_for(std::chrono::seconds(4));
        writer.send(Http::Code::Ok, "Hello, World!");
    }
};

struct FastEvenPagesHandler : public Http::Handler
{
    HTTP_PROTOTYPE(FastEvenPagesHandler)

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        std::string page = request.resource();
        page.erase(0, 1);
        int num = std::stoi(page);
        if (num % 2 != 0)
        {
            std::this_thread::sleep_for(std::chrono::milliseconds(2500));
            writer.send(Http::Code::Ok, std::to_string(num));
        }
        else
        {
            writer.send(Http::Code::Ok, std::to_string(num));
        }
    }
};

struct QueryBounceHandler : public Http::Handler
{
    HTTP_PROTOTYPE(QueryBounceHandler)

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        writer.send(Http::Code::Ok, request.query().as_str());
    }
};

namespace
{
    std::string largeContent(4097, 'a');
}

struct LargeContentHandler : public Http::Handler
{
    HTTP_PROTOTYPE(LargeContentHandler)

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        writer.send(Http::Code::Ok, largeContent);
    }
};

TEST(http_client_test, one_client_with_one_request)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    auto rb       = client.get(server_address);
    auto response = rb.header<Http::Header::Connection>(Http::ConnectionControl::KeepAlive)
                        .send();
    bool done = false;
    response.then(
        [&done](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
                done = true;
        },
        Async::IgnoreException);

    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(done);
}

TEST(http_client_test, one_client_with_multiple_requests)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    const int RESPONSE_SIZE = 3;
    int response_counter    = 0;

    auto rb = client.get(server_address);
    for (int i = 0; i < RESPONSE_SIZE; ++i)
    {
        auto response = rb.send();
        response.then(
            [&response_counter](Http::Response rsp) {
                if (rsp.code() == Http::Code::Ok)
                    ++response_counter;
            },
            Async::IgnoreException);
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);

    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(response_counter == RESPONSE_SIZE);
}

TEST(http_client_test, multiple_clients_with_one_request)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const int CLIENT_SIZE = 3;
    Http::Experimental::Client client1;
    client1.init();
    Http::Experimental::Client client2;
    client2.init();
    Http::Experimental::Client client3;
    client3.init();

    std::vector<Async::Promise<Http::Response>> responses;
    std::atomic<int> response_counter(0);

    auto rb1       = client1.get(server_address);
    auto response1 = rb1.send();
    response1.then(
        [&response_counter](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
                ++response_counter;
        },
        Async::IgnoreException);
    responses.push_back(std::move(response1));
    auto rb2       = client2.get(server_address);
    auto response2 = rb2.send();
    response2.then(
        [&response_counter](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
                ++response_counter;
        },
        Async::IgnoreException);
    responses.push_back(std::move(response2));
    auto rb3       = client3.get(server_address);
    auto response3 = rb3.send();
    response3.then(
        [&response_counter](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
                ++response_counter;
        },
        Async::IgnoreException);
    responses.push_back(std::move(response3));

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);

    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client1.shutdown();
    client2.shutdown();
    client3.shutdown();

    ASSERT_TRUE(response_counter == CLIENT_SIZE);
}

TEST(http_client_test, timeout_reject)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<DelayHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    auto rb       = client.get(server_address).timeout(std::chrono::milliseconds(1000));
    auto response = rb.header<Http::Header::Connection>(Http::ConnectionControl::KeepAlive)
                        .send();
    bool is_reject = false;
    response.then([&is_reject](Http::Response /*rsp*/) { is_reject = false; },
                  [&is_reject](std::exception_ptr /*exc*/) { is_reject = true; });

    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(is_reject);
}

TEST(
    http_client_test,
    one_client_with_multiple_requests_and_one_connection_per_host_and_two_threads)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    auto opts = Http::Experimental::Client::options().maxConnectionsPerHost(1).threads(2);
    client.init(opts);

    std::vector<Async::Promise<Http::Response>> responses;
    const int RESPONSE_SIZE = 6;
    std::atomic<int> response_counter(0);

    auto rb = client.get(server_address);
    for (int i = 0; i < RESPONSE_SIZE; ++i)
    {
        auto response = rb.header<Http::Header::Connection>(Http::ConnectionControl::KeepAlive)
                            .send();
        response.then(
            [&](Http::Response rsp) {
                if (rsp.code() == Http::Code::Ok)
                    ++response_counter;
            },
            Async::IgnoreException);
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);

    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(response_counter == RESPONSE_SIZE);
}

TEST(
    http_client_test,
    one_client_with_multiple_requests_and_two_connections_per_host_and_one_thread)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    auto opts = Http::Experimental::Client::options().maxConnectionsPerHost(2).threads(1);
    client.init(opts);

    std::vector<Async::Promise<Http::Response>> responses;
    const int RESPONSE_SIZE = 6;
    std::atomic<int> response_counter(0);

    auto rb = client.get(server_address);
    for (int i = 0; i < RESPONSE_SIZE; ++i)
    {
        auto response = rb.header<Http::Header::Connection>(Http::ConnectionControl::KeepAlive)
                            .send();
        response.then(
            [&](Http::Response rsp) {
                if (rsp.code() == Http::Code::Ok)
                    ++response_counter;
            },
            Async::IgnoreException);
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);

    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(response_counter == RESPONSE_SIZE);
}

TEST(http_client_test, test_client_timeout)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(4);
    server.init(server_opts);
    server.setHandler(Http::make_handler<FastEvenPagesHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    const int RESPONSE_SIZE         = 4;
    int rejects_counter             = 0;
    const std::vector<int> timeouts = { 0, 1000, 4500, 1000 };

    std::map<int, std::string> res;
    for (int i = 0; i < RESPONSE_SIZE; ++i)
    {
        const std::string page = server_address + "/" + std::to_string(i);
        auto rb                = client.get(page).timeout(std::chrono::milliseconds(timeouts[i]));
        auto response          = rb.send();
        response.then(
            [&res, num = i](Http::Response rsp) {
                if (rsp.code() == Http::Code::Ok)
                {
                    res[num] = rsp.body();
                }
            },
            [&rejects_counter](std::exception_ptr) { ++rejects_counter; });
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);

    barrier.wait_for(std::chrono::seconds(2));

    std::this_thread::sleep_for(std::chrono::seconds(3));

    server.shutdown();
    client.shutdown();

    ASSERT_GE(rejects_counter, 1);
    ASSERT_EQ(res.size(), 2u);

    auto it1 = res.find(0);
    ASSERT_NE(it1, res.end());
    ASSERT_EQ(it1->second, "0");

    auto it2 = res.find(2);
    ASSERT_NE(it2, res.end());
    ASSERT_EQ(it2->second, "2");
}

TEST(http_client_test, client_sends_query)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<QueryBounceHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    std::string queryStr;
    Http::Uri::Query query(
        { { "param1", "1" }, { "param2", "3.14" }, { "param3", "a+string" } });

    auto rb       = client.get(server_address);
    auto response = rb.params(query).send();

    response.then(
        [&queryStr](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
                queryStr = rsp.body();
        },
        Async::IgnoreException);

    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    EXPECT_EQ(queryStr[0], '?');

    std::unordered_map<std::string, std::string> results;
    bool key = true;
    std::string keyStr, valueStr;

    for (auto it = std::next(queryStr.begin()); it != queryStr.end(); it++)
    {
        if (*it == '&' || std::next(it) == queryStr.end())
        {
            if (*it != '&')
                valueStr += *it;
            results[keyStr] = valueStr;
            keyStr          = "";
            valueStr        = "";
            key             = true;
        }
        else if (*it == '=')
            key = false;
        else if (key)
            keyStr += *it;
        else
            valueStr += *it;
    }

    EXPECT_EQ(static_cast<long int>(results.size()),
              std::distance(query.parameters_begin(), query.parameters_end()));

    for (auto entry : results)
    {
        ASSERT_TRUE(query.has(entry.first));
        EXPECT_EQ(entry.second, query.get(entry.first).value());
    }
}

TEST(http_client_test, client_get_large_content)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<LargeContentHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    auto opts = Http::Experimental::Client::options().maxResponseSize(8192);
    client.init(opts);

    auto response = client.get(server_address).send();
    bool done     = false;
    std::string rcvContent;
    response.then(
        [&done, &rcvContent](Http::Response rsp) {
            if (rsp.code() == Http::Code::Ok)
            {
                done       = true;
                rcvContent = rsp.body();
            }
        },
        Async::IgnoreException);

    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(done);
    ASSERT_EQ(largeContent, rcvContent);
}

TEST(http_client_test, client_do_not_get_large_content)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<LargeContentHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    auto opts = Http::Experimental::Client::options().maxResponseSize(4096);
    client.init(opts);

    auto response       = client.get(server_address).send();
    bool ok_flag        = false;
    bool exception_flag = false;
    response.then(
        [&ok_flag](Http::Response /*rsp*/) { ok_flag = true; },
        [&exception_flag](std::exception_ptr /*ptr*/) { exception_flag = true; });

    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_FALSE(ok_flag);
    ASSERT_TRUE(exception_flag);
}
