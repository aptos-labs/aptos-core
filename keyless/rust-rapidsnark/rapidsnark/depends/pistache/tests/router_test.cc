/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* router_test.cc
   Mathieu Stefani, 06 janvier 2016

   Unit tests for the rest router
*/

#include <algorithm>
#include <gtest/gtest.h>

#include <pistache/common.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>
#include <pistache/router.h>

#include <httplib.h>

using namespace Pistache;
using namespace Pistache::Rest;

bool match(const SegmentTreeNode& routes, const std::string& req)
{
    const auto& s = SegmentTreeNode::sanitizeResource(req);
    std::shared_ptr<Route> route;
    std::tie(route, std::ignore, std::ignore) = routes.findRoute({ s.data(), s.size() });
    return route != nullptr;
}

bool matchParams(
    const SegmentTreeNode& routes, const std::string& req,
    std::initializer_list<std::pair<std::string, std::string>> list)
{

    const auto& s = SegmentTreeNode::sanitizeResource(req);
    std::shared_ptr<Route> route;
    std::vector<TypedParam> params;
    std::string_view sv { s.data(), s.length() };
    std::tie(route, params, std::ignore) = routes.findRoute(sv);

    if (route == nullptr)
        return false;

    for (const auto& p : list)
    {
        auto it = std::find_if(
            params.begin(), params.end(),
            [&](const TypedParam& param) { return param.name() == p.first; });
        if (it == std::end(params))
            return false;
        if (it->as<std::string>() != p.second)
            return false;
    }
    return true;
}

bool matchSplat(const SegmentTreeNode& routes, const std::string& req,
                std::initializer_list<std::string> list)
{

    const auto& s = SegmentTreeNode::sanitizeResource(req);
    std::shared_ptr<Route> route;
    std::vector<TypedParam> splats;
    std::string_view sv { s.data(), s.length() };
    std::tie(route, std::ignore, splats) = routes.findRoute(sv);

    if (route == nullptr)
        return false;

    if (list.size() != splats.size())
        return false;

    size_t i = 0;
    for (const auto& s : list)
    {
        auto splat = splats[i].as<std::string>();
        if (splat != s)
            return false;
        ++i;
    }

    return true;
}

TEST(router_test, test_fixed_routes)
{
    SegmentTreeNode routes;
    auto s = SegmentTreeNode::sanitizeResource("/v1/hello");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);

    ASSERT_TRUE(match(routes, "/v1/hello"));
    ASSERT_FALSE(match(routes, "/v2/hello"));
    ASSERT_FALSE(match(routes, "/v1/hell0"));

    s = SegmentTreeNode::sanitizeResource("/a/b/c");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);
    ASSERT_TRUE(match(routes, "/a/b/c"));
}

TEST(router_test, test_parameters)
{
    SegmentTreeNode routes;
    const auto& s = SegmentTreeNode::sanitizeResource("/v1/hello/:name/");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);

    ASSERT_TRUE(matchParams(routes, "/v1/hello/joe", { { ":name", "joe" } }));

    const auto& p = SegmentTreeNode::sanitizeResource("/greetings/:from/:to");
    routes.addRoute(std::string_view { p.data(), p.length() }, nullptr, nullptr);
    ASSERT_TRUE(matchParams(routes, "/greetings/foo/bar",
                            { { ":from", "foo" }, { ":to", "bar" } }));
}

TEST(router_test, test_optional)
{
    SegmentTreeNode routes;
    auto s = SegmentTreeNode::sanitizeResource("/get/:key?/bar");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);

    ASSERT_FALSE(matchParams(routes, "/get/bar", { { ":key", "whatever" } }));
    ASSERT_TRUE(matchParams(routes, "/get/foo/bar", { { ":key", "foo" } }));
}

TEST(router_test, test_splat)
{
    SegmentTreeNode routes;
    auto s = SegmentTreeNode::sanitizeResource("/say/*/to/*");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);

    ASSERT_TRUE(match(routes, "/say/hello/to/user"));
    ASSERT_FALSE(match(routes, "/say/hello/to"));
    ASSERT_FALSE(match(routes, "/say/hello/to/user/please"));

    ASSERT_TRUE(matchSplat(routes, "/say/hello/to/user", { "hello", "user" }));
    ASSERT_TRUE(matchSplat(routes, "/say/hello/to/user/", { "hello", "user" }));
}

TEST(router_test, test_sanitize)
{
    SegmentTreeNode routes;
    auto s = SegmentTreeNode::sanitizeResource("//v1//hello/");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);

    ASSERT_TRUE(match(routes, "/v1/hello////"));
}

TEST(router_test, test_mixed)
{
    SegmentTreeNode routes;
    auto s = SegmentTreeNode::sanitizeResource("/hello");
    auto p = SegmentTreeNode::sanitizeResource("/*");
    routes.addRoute(std::string_view { s.data(), s.length() }, nullptr, nullptr);
    routes.addRoute(std::string_view { p.data(), p.length() }, nullptr, nullptr);

    ASSERT_TRUE(match(routes, "/hello"));
    ASSERT_TRUE(match(routes, "/hi"));

    ASSERT_FALSE(matchSplat(routes, "/hello", { "hello" }));
    ASSERT_TRUE(matchSplat(routes, "/hi", { "hi" }));
}

TEST(router_test, test_notfound_exactly_once)
{
    Address addr(Ipv4::any(), 0);
    auto endpoint = std::make_shared<Http::Endpoint>(addr);

    auto opts = Http::Endpoint::options().threads(1).maxRequestSize(4096);
    endpoint->init(opts);

    int count_found     = 0;
    int count_not_found = 0;

    Rest::Router router;
    Routes::NotFound(router, [&count_not_found](const Pistache::Rest::Request& request, Pistache::Http::ResponseWriter response) {
        count_not_found++;
        std::string err { "Couldn't find route: \"" + request.resource() + "\"\n" };
        response.send(Pistache::Http::Code::Not_Found, err);
        return Pistache::Rest::Route::Result::Ok;
    });
    Routes::Get(router, "/moogle",
                [&count_found](const Pistache::Rest::Request&,
                               Pistache::Http::ResponseWriter response) {
                    count_found++;
                    response.send(Pistache::Http::Code::Ok, "kupo!\n");
                    return Pistache::Rest::Route::Result::Ok;
                });

    endpoint->setHandler(router.handler());
    endpoint->serveThreaded();
    const auto bound_port = endpoint->getPort();
    httplib::Client client("localhost", bound_port);

    // Verify that the notFound handler is NOT called when route is found.
    count_not_found = count_found = 0;
    client.Get("/moogle");
    ASSERT_EQ(count_found, 1);
    ASSERT_EQ(count_not_found, 0);

    // Verify simple solution to bug #323 (one bad url triggered 2 routes).
    count_not_found = count_found = 0;
    client.Get("/kefka");
    ASSERT_EQ(count_found, 0);
    ASSERT_EQ(count_not_found, 1);

    // Anal test, 2 calls = 2 route hits.
    count_not_found = count_found = 0;
    client.Get("/vicks");
    client.Get("/wedge");
    ASSERT_EQ(count_found, 0);
    ASSERT_EQ(count_not_found, 2);

    endpoint->shutdown();
}

TEST(router_test, test_route_head_request)
{
    Address addr(Ipv4::any(), 0);
    auto endpoint = std::make_shared<Http::Endpoint>(addr);

    auto opts = Http::Endpoint::options().threads(1).maxRequestSize(4096);
    endpoint->init(opts);

    int count_found = 0;

    Rest::Router router;

    Routes::Head(router, "/moogle",
                 [&count_found](const Pistache::Rest::Request&,
                                Pistache::Http::ResponseWriter response) {
                     count_found++;
                     response.send(Pistache::Http::Code::Ok);
                     return Pistache::Rest::Route::Result::Ok;
                 });

    endpoint->setHandler(router.handler());
    endpoint->serveThreaded();
    const auto bound_port = endpoint->getPort();
    httplib::Client client("localhost", bound_port);

    count_found = 0;
    client.Head("/moogle");
    ASSERT_EQ(count_found, 1);

    endpoint->shutdown();
}

class MyHandler
{
public:
    MyHandler() = default;

    void handle(
        const Pistache::Rest::Request&,
        Pistache::Http::ResponseWriter response)
    {
        count_++;
        response.send(Pistache::Http::Code::Ok);
    }

    int getCount() { return count_; }

private:
    int count_ = 0;
};

TEST(router_test, test_bind_shared_ptr)
{
    Address addr(Ipv4::any(), 0);
    auto endpoint = std::make_shared<Http::Endpoint>(addr);

    auto opts = Http::Endpoint::options().threads(1).maxRequestSize(4096);
    endpoint->init(opts);

    std::shared_ptr<MyHandler> sharedPtr = std::make_shared<MyHandler>();

    Rest::Router router;

    Routes::Head(router, "/tinkywinky", Routes::bind(&MyHandler::handle, sharedPtr));

    endpoint->setHandler(router.handler());
    endpoint->serveThreaded();
    const auto bound_port = endpoint->getPort();
    httplib::Client client("localhost", bound_port);

    ASSERT_EQ(sharedPtr->getCount(), 0);
    client.Head("/tinkywinky");
    ASSERT_EQ(sharedPtr->getCount(), 1);

    endpoint->shutdown();
}

class HandlerWithAuthMiddleware : public MyHandler
{
public:
    HandlerWithAuthMiddleware() = default;

    bool do_auth(Pistache::Http::Request& request, Pistache::Http::ResponseWriter& response)
    {
        auth_count++;
        try
        {
            auto auth = request.headers().get<Pistache::Http::Header::Authorization>();
            if (auth->getMethod() == Pistache::Http::Header::Authorization::Method::Basic)
            {
                auth_succ_count++;
                return true;
            }
            else
            {
                response.send(Pistache::Http::Code::Unauthorized);
                return false;
            }
        }
        catch (std::runtime_error&)
        {
            return false;
        }
    }

    int getAuthCount() { return auth_count; }
    int getSuccAuthCount() { return auth_succ_count; }

private:
    int auth_count      = 0;
    int auth_succ_count = 0;
};

bool fill_auth_header(Pistache::Http::Request& request, Pistache::Http::ResponseWriter& /*response*/)
{
    auto au = Pistache::Http::Header::Authorization();
    au.setBasicUserPassword("foo", "bar");
    request.headers().add<decltype(au)>(au);
    return true;
}

bool stop_processing(Pistache::Http::Request& /*request*/, Pistache::Http::ResponseWriter& response)
{
    response.send(Pistache::Http::Code::No_Content);
    return false;
}

TEST(router_test, test_middleware_stop_processing)
{
    Address addr(Ipv4::any(), 0);
    auto endpoint = std::make_shared<Http::Endpoint>(addr);

    auto opts = Http::Endpoint::options().threads(1);
    endpoint->init(opts);

    auto sharedPtr = std::make_shared<HandlerWithAuthMiddleware>();

    Rest::Router router;
    router.addMiddleware(Routes::middleware(&stop_processing));
    Routes::Head(router, "/tinkywinky", Routes::bind(&HandlerWithAuthMiddleware::handle, sharedPtr));
    endpoint->setHandler(router.handler());
    endpoint->serveThreaded();

    const auto bound_port = endpoint->getPort();
    httplib::Client client("localhost", bound_port);

    ASSERT_EQ(sharedPtr->getCount(), 0);
    auto response = client.Head("/tinkywinky");
    ASSERT_EQ(sharedPtr->getCount(), 0);
    ASSERT_EQ(response->status, int(Pistache::Http::Code::No_Content));
}

TEST(router_test, test_auth_middleware)
{
    Address addr(Ipv4::any(), 0);
    auto endpoint = std::make_shared<Http::Endpoint>(addr);

    auto opts = Http::Endpoint::options().threads(1);
    endpoint->init(opts);

    HandlerWithAuthMiddleware handler;

    Rest::Router router;
    router.addMiddleware(Routes::middleware(&fill_auth_header));
    router.addMiddleware(Routes::middleware(&HandlerWithAuthMiddleware::do_auth, &handler));

    Routes::Head(router, "/tinkywinky", Routes::bind(&HandlerWithAuthMiddleware::handle, &handler));
    endpoint->setHandler(router.handler());
    endpoint->serveThreaded();

    const auto bound_port = endpoint->getPort();
    httplib::Client client("localhost", bound_port);

    ASSERT_EQ(handler.getCount(), 0);
    auto response = client.Head("/tinkywinky");
    ASSERT_EQ(handler.getCount(), 1);
    ASSERT_EQ(handler.getAuthCount(), 1);
    ASSERT_EQ(handler.getSuccAuthCount(), 1);
    ASSERT_EQ(response->status, int(Pistache::Http::Code::Ok));
}

TEST(segment_tree_node_test, test_resource_sanitize)
{
    ASSERT_EQ(SegmentTreeNode::sanitizeResource("/path"), "path");
    ASSERT_EQ(SegmentTreeNode::sanitizeResource("/path/to/bar"), "path/to/bar");
    ASSERT_EQ(SegmentTreeNode::sanitizeResource("/path//to/bar"), "path/to/bar");
    ASSERT_EQ(SegmentTreeNode::sanitizeResource("/path//to/bar"), "path/to/bar");
    ASSERT_EQ(SegmentTreeNode::sanitizeResource("/path/to///////:place"), "path/to/:place");
}

namespace
{
    class WaitHelper
    {
    public:
        void increment()
        {
            std::lock_guard<std::mutex> lock(counterLock_);
            ++counter_;
            cv_.notify_one();
        }

        template <typename Duration>
        bool wait(const size_t count, const Duration timeout)
        {
            std::unique_lock<std::mutex> lock(counterLock_);
            return cv_.wait_for(lock, timeout,
                                [this, count]() { return counter_ >= count; });
        }

    private:
        size_t counter_ = 0;
        std::mutex counterLock_;
        std::condition_variable cv_;
    };

    TEST(router_test, test_client_disconnects)
    {
        Address addr(Ipv4::any(), 0);
        auto endpoint = std::make_shared<Http::Endpoint>(addr);

        auto opts = Http::Endpoint::options().threads(1).maxRequestSize(4096);
        endpoint->init(opts);

        int count_found = 0;
        WaitHelper count_disconnect;

        Rest::Router router;

        Routes::Head(router, "/moogle",
                     [&count_found](const Pistache::Rest::Request&,
                                    Pistache::Http::ResponseWriter response) {
                         count_found++;
                         response.send(Pistache::Http::Code::Ok);
                         return Pistache::Rest::Route::Result::Ok;
                     });

        router.addDisconnectHandler(
            [&count_disconnect](const std::shared_ptr<Tcp::Peer>&) {
                count_disconnect.increment();
            });

        endpoint->setHandler(router.handler());
        endpoint->serveThreaded();
        const auto bound_port = endpoint->getPort();
        {
            httplib::Client client("localhost", bound_port);
            count_found = 0;
            client.Head("/moogle");
            ASSERT_EQ(count_found, 1);
        }

        const bool result = count_disconnect.wait(1, std::chrono::seconds(2));

        endpoint->shutdown();
        ASSERT_EQ(result, 1);
    }
} // namespace
