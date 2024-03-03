/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/async.h>
#include <pistache/client.h>
#include <pistache/cookie.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>

#include <gtest/gtest.h>

#include <chrono>
#include <string>
#include <unordered_map>

using namespace Pistache;

struct CookieHandler : public Http::Handler
{
    HTTP_PROTOTYPE(CookieHandler)

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter response) override
    {
        // Synthetic behaviour, just for testing purposes
        for (auto&& cookie : request.cookies())
        {
            response.cookies().add(cookie);
        }
        response.send(Http::Code::Ok, "Ok");
    }
};

TEST(http_client_test, one_client_with_one_request_with_onecookie)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<CookieHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    const std::string name  = "FOO";
    const std::string value = "bar";
    auto cookie             = Http::Cookie(name, value);
    auto rb                 = client.get(server_address).cookie(cookie);
    auto response           = rb.send();

    Http::CookieJar cj;
    response.then([&](Http::Response rsp) { cj = rsp.cookies(); },
                  Async::IgnoreException);
    responses.push_back(std::move(response));

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_TRUE(cj.has(name));
}

TEST(http_client_test, one_client_with_one_request_with_several_cookies)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<CookieHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    const std::string name1  = "FOO";
    const std::string value1 = "bar";
    auto cookie1             = Http::Cookie(name1, value1);
    const std::string name2  = "FIZZ";
    const std::string value2 = "Buzz";
    auto cookie2             = Http::Cookie(name2, value2);
    const std::string name3  = "Key";
    const std::string value3 = "value";
    auto cookie3             = Http::Cookie(name3, value3);
    auto rb                  = client.get(server_address)
                  .cookie(cookie1)
                  .cookie(cookie2)
                  .cookie(cookie3);
    auto response = rb.send();

    std::unordered_map<std::string, std::string> cookiesStorages;
    response.then(
        [&](Http::Response rsp) {
            for (auto&& cookie : rsp.cookies())
            {
                cookiesStorages[cookie.name] = cookie.value;
            }
        },
        Async::IgnoreException);
    responses.push_back(std::move(response));

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);
    barrier.wait_for(std::chrono::seconds(5));

    server.shutdown();
    client.shutdown();

    ASSERT_NE(cookiesStorages.find(name1), cookiesStorages.end());
    ASSERT_EQ(cookiesStorages[name1], value1);
    ASSERT_NE(cookiesStorages.find(name2), cookiesStorages.end());
    ASSERT_EQ(cookiesStorages[name2], value2);
    ASSERT_NE(cookiesStorages.find(name3), cookiesStorages.end());
    ASSERT_EQ(cookiesStorages[name3], value3);
}
