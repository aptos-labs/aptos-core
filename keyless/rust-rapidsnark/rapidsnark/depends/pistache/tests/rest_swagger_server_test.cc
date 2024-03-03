/*
 * SPDX-FileCopyrightText: 2022 Kirill Efimov
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>

#include <pistache/description.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>
#include <pistache/peer.h>
#include <pistache/router.h>
#include <pistache/serializer/rapidjson.h>

#include <httplib.h>

#if __has_include(<filesystem>)
#include <filesystem>
namespace filesystem = std::filesystem;
#else
#include <experimental/filesystem>
namespace filesystem = std::experimental::filesystem;
#endif

using namespace std;
using namespace Pistache;

class SwaggerEndpoint
{
public:
    SwaggerEndpoint(Address addr)
        : httpEndpoint(make_shared<Http::Endpoint>(addr))
        , desc("SwaggerEndpoint API", "1.0")
    { }

    void init()
    {
        auto opts = Http::Endpoint::options().threads(1);
        httpEndpoint->init(opts);
    }

    void start()
    {
        router.initFromDescription(desc);

        Rest::Swagger swagger(desc);
        swagger
            .uiPath("/doc")
            .uiDirectory(filesystem::current_path() / "assets")
            .apiPath("/banker-api.json")
            .serializer(&Rest::Serializer::rapidJson)
            .install(router);

        httpEndpoint->setHandler(router.handler());
        httpEndpoint->serve();
    }

    void shutdown() { httpEndpoint->shutdown(); }

    Port getPort() const { return httpEndpoint->getPort(); }

private:
    shared_ptr<Http::Endpoint> httpEndpoint;
    Rest::Description desc;
    Rest::Router router;
};

TEST(rest_swagger_server_test, basic_test)
{
    filesystem::create_directory("assets");

    ofstream("assets/good.txt") << "good";

    ofstream("bad.txt") << "bad";

    Address addr(Ipv4::loopback(), Port(0));
    SwaggerEndpoint swagger(addr);

    swagger.init();
    thread t([&swagger]() {
        while (swagger.getPort() == 0)
        {
            this_thread::yield();
        }

        Port port = swagger.getPort();

        cout << "CWD = " << filesystem::current_path() << endl;
        cout << "Port = " << port << endl;

        httplib::Client client("localhost", port);

        // Test if we have access to files inside the UI folder.
        auto goodRes = client.Get("/doc/good.txt");
        // Attempt to read file outside of the UI directory should fail even if
        // the file exists.
        client.set_connection_timeout(1000);
        client.set_read_timeout(1000);
        auto badRes = client.Get("/doc/../bad.txt");
        // Ensure the server is shut down before calling asserts that could
        // terminate the thread without cleaning up
        swagger.shutdown();

        ASSERT_EQ(goodRes->status, 200);
        ASSERT_EQ(goodRes->body, "good");

        ASSERT_EQ(badRes->status, 404);
        ASSERT_NE(badRes->body, "bad");
    });
    swagger.start();

    t.join();
    filesystem::remove_all("assets");
    filesystem::remove_all("bad.txt");
}
