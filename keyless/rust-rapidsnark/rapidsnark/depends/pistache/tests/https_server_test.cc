/*
 * SPDX-FileCopyrightText: 2019 Louis Solofrizzo
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <array>
#include <cstring>

#include <pistache/client.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>

#include <gtest/gtest.h>

#include <curl/curl.h>

using namespace Pistache;

/* Should these tests fail, please re-run "./new-certs.sh" from the "./certs"
 * directory.
 */

static size_t write_cb(void* contents, size_t size, size_t nmemb, void* userp)
{
    (static_cast<std::string*>(userp))->append(static_cast<char*>(contents), size * nmemb);
    return size * nmemb;
}

static std::string getServerUrl(const Http::Endpoint& server)
{
    return std::string("https://localhost:") + server.getPort().toString();
}

struct HelloHandler : public Http::Handler
{
    HTTP_PROTOTYPE(HelloHandler)

    void onRequest(const Http::Request&, Http::ResponseWriter writer) override
    {
        writer.send(Http::Code::Ok, "Hello, World!");
    }
};

struct ServeFileHandler : public Http::Handler
{
    HTTP_PROTOTYPE(ServeFileHandler)

    void onRequest(const Http::Request&, Http::ResponseWriter writer) override
    {
        Http::serveFile(writer, "./certs/rootCA.crt")
            .then(
                [](ssize_t bytes) {
                    std::cout << "Sent " << bytes << " bytes" << std::endl;
                },
                Async::NoExcept);
    }
};

TEST(https_server_test, basic_tls_request)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer, "Hello, World!");
}

TEST(https_server_test, basic_tls_request_with_chained_server_cert)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server_from_intermediate_with_chain.crt",
                  "./certs/server_from_intermediate.key");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer, "Hello, World!");
}

TEST(https_server_test, basic_tls_request_with_auth)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.useSSLAuth("./certs/rootCA.crt");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_SSLCERT, "./certs/client.crt");
    curl_easy_setopt(curl, CURLOPT_SSLKEY, "./certs/client.key");
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");

    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer, "Hello, World!");
}

TEST(https_server_test, basic_tls_request_with_auth_no_client_cert)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.useSSLAuth("./certs/rootCA.crt");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");

    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_NE(res, CURLE_OK);
}

TEST(https_server_test, basic_tls_request_with_auth_client_cert_not_signed)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.useSSLAuth("./certs/rootCA.crt");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_SSLCERT, "./certs/client_not_signed.crt");
    curl_easy_setopt(curl, CURLOPT_SSLKEY, "./certs/client_not_signed.key");
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");

    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_NE(res, CURLE_OK);
}

static bool callback_called = false;
static int verify_callback(int verify, void* ctx)
{
    (void)verify;
    (void)ctx;

    callback_called = true;
    return 1;
}

TEST(https_server_test, basic_tls_request_with_auth_with_cb)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.useSSLAuth("./certs/rootCA.crt", "./certs", &verify_callback);
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_SSLCERT, "./certs/client.crt");
    curl_easy_setopt(curl, CURLOPT_SSLKEY, "./certs/client.key");
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");

    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer, "Hello, World!");
    ASSERT_EQ(callback_called, true);
    callback_called = false;
}

TEST(https_server_test, basic_tls_request_with_servefile)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);

    server.init(server_opts);
    server.setHandler(Http::make_handler<ServeFileHandler>());
    server.useSSL("./certs/server.crt", "./certs/server.key");
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    std::array<char, CURL_ERROR_SIZE> errorstring;
    curl_easy_setopt(curl, CURLOPT_ERRORBUFFER, errorstring.data());
    // curl_easy_setopt(curl, CURLOPT_VERBOSE, true);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    if (res != CURLE_OK)
    {
        std::cerr << errorstring.data() << std::endl;
    }

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer.rfind("-----BEGIN CERTIFICATE-----", 0), 0u);
}

TEST(https_server_test, basic_tls_request_with_password_cert)
{
    Http::Endpoint server(Address("localhost", Pistache::Port(0)));

    const auto passwordCallback = [](char* buf, int size, int /*rwflag*/, void* /*u*/) -> int
    {
        static constexpr const char* const password = "test";
        std::strncpy(buf, password, size);
        return static_cast<int>(std::strlen(password));
    };

    server.init(Http::Endpoint::options().flags(Tcp::Options::ReuseAddr));
    server.setHandler(Http::make_handler<HelloHandler>());
    server.useSSL("./certs/server_protected.crt", "./certs/server_protected.key", false, passwordCallback);
    server.serveThreaded();

    CURL* curl;
    CURLcode res;
    std::string buffer;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    ASSERT_NE(curl, nullptr);

    const auto url = getServerUrl(server);
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_CAINFO, "./certs/rootCA.crt");
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, &write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buffer);

    /* Skip hostname check */
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0L);

    res = curl_easy_perform(curl);

    curl_easy_cleanup(curl);
    curl_global_cleanup();

    server.shutdown();

    ASSERT_EQ(res, CURLE_OK);
    ASSERT_EQ(buffer, "Hello, World!");
}
