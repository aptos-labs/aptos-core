/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/async.h>
#include <pistache/client.h>
#include <pistache/common.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>
#include <pistache/peer.h>

#include <gtest/gtest.h>

#include <chrono>
#include <condition_variable>
#include <fstream>
#include <future>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>

#include "tcp_client.h"

using namespace Pistache;

namespace
{

    class SimpleLogger
    {
    public:
        static SimpleLogger& instance()
        {
            static SimpleLogger logger;
            return logger;
        }

        void log(const std::string& message)
        {
            std::lock_guard<std::mutex> guard { m_coutLock };
            std::cout << message << std::endl;
        }

    private:
        SimpleLogger() = default;
        std::mutex m_coutLock;
    };

    // from
    // https://stackoverflow.com/questions/9667963/can-i-rewrite-a-logging-macro-with-stream-operators-to-use-a-c-template-functi
    class ScopedLogger
    {
    public:
        ScopedLogger(const std::string& prefix)
        {
            m_stream << "[" << prefix << "] [" << std::hex << std::this_thread::get_id() << "] " << std::dec;
        }

        ~ScopedLogger()
        {
            SimpleLogger::instance().log(m_stream.str());
        }

        std::stringstream& stream()
        {
            return m_stream;
        }

    private:
        std::stringstream m_stream;
    };

#define LOGGER(prefix, message) ScopedLogger(prefix).stream() << message;

}

struct HelloHandlerWithDelay : public Http::Handler
{
    HTTP_PROTOTYPE(HelloHandlerWithDelay)

    explicit HelloHandlerWithDelay(int delay = 0)
        : delay_(delay)
    {
        LOGGER("server", "Init Hello handler with " << delay_ << " seconds delay");
    }

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        std::this_thread::sleep_for(std::chrono::seconds(delay_));
        writer.send(Http::Code::Ok, "Hello, World!");
    }

    int delay_;
};

constexpr char SLOW_PAGE[] = "/slowpage";

struct HandlerWithSlowPage : public Http::Handler
{
    HTTP_PROTOTYPE(HandlerWithSlowPage)

    explicit HandlerWithSlowPage(int delay = 0)
        : delay_(delay)
    { }

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        std::string message;
        if (request.resource() == SLOW_PAGE)
        {
            std::this_thread::sleep_for(std::chrono::seconds(delay_));
            message = "[" + std::to_string(counter++) + "] Slow page content!";
        }
        else
        {
            message = "[" + std::to_string(counter++) + "] Hello, World!";
        }

        writer.send(Http::Code::Ok, message);
        LOGGER("server", "Sent: " << message);
    }

    int delay_;
    static std::atomic<size_t> counter;
};

std::atomic<size_t> HandlerWithSlowPage::counter { 0 };

struct FileHandler : public Http::Handler
{
    HTTP_PROTOTYPE(FileHandler)

    explicit FileHandler(const std::string& fileName)
        : fileName_(fileName)
    { }

    void onRequest(const Http::Request& /*request*/,
                   Http::ResponseWriter writer) override
    {
        Http::serveFile(writer, fileName_)
            .then(
                [this](ssize_t bytes) {
                    LOGGER("server", "Sent " << bytes << " bytes from " << fileName_ << " file");
                },
                Async::IgnoreException);
    }

private:
    std::string fileName_;
};

struct AddressEchoHandler : public Http::Handler
{
    HTTP_PROTOTYPE(AddressEchoHandler)

    AddressEchoHandler() { }

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        std::string requestAddress = request.address().host();
        writer.send(Http::Code::Ok, requestAddress);
        LOGGER("server", "Sent: " << requestAddress);
    }
};

constexpr const char* ExpectedResponseLine = "HTTP/1.1 408 Request Timeout";

struct PingHandler : public Http::Handler
{
    HTTP_PROTOTYPE(PingHandler)

    PingHandler() = default;

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        if (request.resource() == "/ping")
        {
            writer.send(Http::Code::Ok, "PONG");
        }
        else
        {
            writer.send(Http::Code::Not_Found);
        }
    }
};

int clientLogicFunc(size_t response_size, const std::string& server_page,
                    int timeout_seconds, int wait_seconds)
{
    Http::Experimental::Client client;
    client.init();

    std::vector<Async::Promise<Http::Response>> responses;
    auto rb              = client.get(server_page).timeout(std::chrono::seconds(timeout_seconds));
    int resolver_counter = 0;
    int reject_counter   = 0;
    for (size_t i = 0; i < response_size; ++i)
    {
        auto response = rb.send();
        response.then(
            [&resolver_counter, pos = i](Http::Response resp) {
                if (resp.code() == Http::Code::Ok)
                {
                    LOGGER("client", "[" << pos << "] Response: " << resp.code() << ", body: `" << resp.body() << "`");
                    ++resolver_counter;
                }
                else
                {
                    LOGGER("client", "[" << pos << "] Response: " << resp.code());
                }
            },
            [&reject_counter, pos = i](std::exception_ptr exc) {
                PrintException excPrinter;
                LOGGER("client", "[" << pos << "] Reject with reason:");
                excPrinter(exc);
                ++reject_counter;
            });
        responses.push_back(std::move(response));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);
    barrier.wait_for(std::chrono::seconds(wait_seconds));

    client.shutdown();

    LOGGER("client", "resolves: " << resolver_counter << ", rejects: " << reject_counter << ", request timeout: " << timeout_seconds << " seconds"
                                  << ", wait: " << wait_seconds << " seconds");

    return resolver_counter;
}

TEST(http_server_test,
     client_disconnection_on_timeout_from_single_threaded_server)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);

    LOGGER("test", "Trying to run server...");
    const int ONE_SECOND_TIMEOUT = 1;
    const int SIX_SECONDS_DELAY  = 6;
    server.setHandler(
        Http::make_handler<HelloHandlerWithDelay>(SIX_SECONDS_DELAY));
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    const int CLIENT_REQUEST_SIZE = 1;
    int counter                   = clientLogicFunc(CLIENT_REQUEST_SIZE, server_address,
                                  ONE_SECOND_TIMEOUT, SIX_SECONDS_DELAY);

    server.shutdown();

    ASSERT_EQ(counter, 0);
}

TEST(
    http_server_test,
    client_multiple_requests_disconnection_on_timeout_from_single_threaded_server)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);

    LOGGER("test", "Trying to run server...");
    const int ONE_SECOND_TIMEOUT = 1;
    const int SIX_SECONDS_DELAY  = 6;
    server.setHandler(
        Http::make_handler<HelloHandlerWithDelay>(SIX_SECONDS_DELAY));
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    const int CLIENT_REQUEST_SIZE = 3;
    int counter                   = clientLogicFunc(CLIENT_REQUEST_SIZE, server_address,
                                  ONE_SECOND_TIMEOUT, SIX_SECONDS_DELAY);

    server.shutdown();

    ASSERT_EQ(counter, 0);
}

TEST(http_server_test, multiple_client_with_requests_to_multithreaded_server)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(3);
    server.init(server_opts);
    LOGGER("test", "Trying to run server...");
    server.setHandler(Http::make_handler<HelloHandlerWithDelay>());
    ASSERT_NO_THROW(server.serveThreaded());

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    const int NO_TIMEOUT                = 0;
    const int SIX_SECONDS_TIMOUT        = 6;
    const int FIRST_CLIENT_REQUEST_SIZE = 4;
    std::future<int> result1(std::async(clientLogicFunc,
                                        FIRST_CLIENT_REQUEST_SIZE, server_address,
                                        NO_TIMEOUT, SIX_SECONDS_TIMOUT));
    const int SECOND_CLIENT_REQUEST_SIZE = 5;
    std::future<int> result2(
        std::async(clientLogicFunc, SECOND_CLIENT_REQUEST_SIZE, server_address,
                   NO_TIMEOUT, SIX_SECONDS_TIMOUT));

    int res1 = result1.get();
    int res2 = result2.get();

    server.shutdown();

    ASSERT_EQ(res1, FIRST_CLIENT_REQUEST_SIZE);
    ASSERT_EQ(res2, SECOND_CLIENT_REQUEST_SIZE);
}

TEST(http_server_test,
     multiple_client_with_different_requests_to_multithreaded_server)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags).threads(4);
    server.init(server_opts);
    const int SIX_SECONDS_DELAY = 6;
    server.setHandler(Http::make_handler<HandlerWithSlowPage>(SIX_SECONDS_DELAY));
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    const int FIRST_CLIENT_REQUEST_SIZE = 1;
    const int FIRST_CLIENT_TIMEOUT      = SIX_SECONDS_DELAY / 2;
    std::future<int> result1(std::async(
        clientLogicFunc, FIRST_CLIENT_REQUEST_SIZE, server_address + SLOW_PAGE,
        FIRST_CLIENT_TIMEOUT, SIX_SECONDS_DELAY));
    const int SECOND_CLIENT_REQUEST_SIZE = 2;
    const int SECOND_CLIENT_TIMEOUT      = SIX_SECONDS_DELAY * 2;
    std::future<int> result2(
        std::async(clientLogicFunc, SECOND_CLIENT_REQUEST_SIZE, server_address,
                   SECOND_CLIENT_TIMEOUT, 2 * SIX_SECONDS_DELAY));

    int res1 = result1.get();
    int res2 = result2.get();

    server.shutdown();

    if (hardware_concurrency() > 1)
    {
        ASSERT_EQ(res1, 0);
        ASSERT_EQ(res2, SECOND_CLIENT_REQUEST_SIZE);
    }
    else
    {
        ASSERT_TRUE(true);
    }
}

TEST(http_server_test, server_with_static_file)
{
    const std::string data("Hello, World!");
    char fileName[PATH_MAX] = "/tmp/pistacheioXXXXXX";
    if (!mkstemp(fileName))
    {
        std::cerr << "No suitable filename can be generated!" << std::endl;
    }
    LOGGER("test", "Creating temporary file: " << fileName);

    std::ofstream tmpFile;
    tmpFile.open(fileName);
    tmpFile << data;
    tmpFile.close();

    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<FileHandler>(fileName));
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    Http::Experimental::Client client;
    client.init();
    auto rb       = client.get(server_address);
    auto response = rb.send();
    std::string resultData;
    response.then(
        [&resultData](Http::Response resp) {
            std::cout << "Response code is " << resp.code() << std::endl;
            if (resp.code() == Http::Code::Ok)
            {
                resultData = resp.body();
            }
        },
        Async::Throw);

    const int WAIT_TIME = 2;
    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(WAIT_TIME));

    client.shutdown();
    server.shutdown();

    LOGGER("test", "Deleting file " << fileName);
    std::remove(fileName);

    ASSERT_EQ(data, resultData);
}

TEST(http_server_test, server_request_copies_address)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<AddressEchoHandler>());
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    Http::Experimental::Client client;
    client.init();
    auto rb       = client.get(server_address);
    auto response = rb.send();
    std::string resultData;
    response.then(
        [&resultData](Http::Response resp) {
            LOGGER("client", " Response code is " << resp.code());
            if (resp.code() == Http::Code::Ok)
            {
                resultData = resp.body();
            }
        },
        Async::Throw);

    const int WAIT_TIME = 2;
    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(WAIT_TIME));

    client.shutdown();
    server.shutdown();

    ASSERT_EQ("127.0.0.1", resultData);
}

struct ResponseSizeHandler : public Http::Handler
{
    HTTP_PROTOTYPE(ResponseSizeHandler)

    explicit ResponseSizeHandler(size_t& rsize, Http::Code& rcode)
        : rsize_(rsize)
        , rcode_(rcode)
    { }

    void onRequest(const Http::Request& request,
                   Http::ResponseWriter writer) override
    {
        std::string requestAddress = request.address().host();
        writer.send(Http::Code::Ok, requestAddress);
        LOGGER("server", "Sent: " << requestAddress);
        rsize_ = writer.getResponseSize();
        rcode_ = writer.getResponseCode();
    }

    size_t& rsize_;
    Http::Code& rcode_;
};

TEST(http_server_test, response_size_captured)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    size_t rsize = 0;
    Http::Code rcode;

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);
    server.setHandler(Http::make_handler<ResponseSizeHandler>(rsize, rcode));
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    LOGGER("test", "Server address: " << server_address);

    // Use the built-in http client, but this test is interested in testing
    // that the ResponseWriter in the server stashed the correct size and code
    // values.
    Http::Experimental::Client client;
    client.init();
    auto rb       = client.get(server_address);
    auto response = rb.send();
    std::string resultData;
    response.then(
        [&resultData](Http::Response resp) {
            LOGGER("client", "Response code is " << resp.code());
            if (resp.code() == Http::Code::Ok)
            {
                resultData = resp.body();
            }
        },
        Async::Throw);

    const int WAIT_TIME = 2;
    Async::Barrier<Http::Response> barrier(response);
    barrier.wait_for(std::chrono::seconds(WAIT_TIME));

    client.shutdown();
    server.shutdown();

    // Sanity check (stolen from AddressEchoHandler test).
    ASSERT_EQ("127.0.0.1", resultData);

    LOGGER("test", "Response size is " << rsize);
    ASSERT_GT(rsize, 1u);
    ASSERT_LT(rsize, 300u);
    ASSERT_EQ(rcode, Http::Code::Ok);
}

TEST(http_server_test, client_request_timeout_on_only_connect_raises_http_408)
{
    Pistache::Address address("localhost", Pistache::Port(0));

    const auto headerTimeout = std::chrono::seconds(2);

    Http::Endpoint server(address);
    auto flags = Tcp::Options::ReuseAddr;
    auto opts  = Http::Endpoint::options()
                    .flags(flags)
                    .headerTimeout(headerTimeout);

    server.init(opts);
    server.setHandler(Http::make_handler<PingHandler>());
    server.serveThreaded();

    auto port = server.getPort();
    auto addr = "localhost:" + port.toString();
    LOGGER("test", "Server address: " << addr)

    TcpClient client;
    EXPECT_TRUE(client.connect(Pistache::Address("localhost", port))) << client.lastError();

    char recvBuf[1024] = {
        0,
    };
    size_t bytes;
    EXPECT_TRUE(client.receive(recvBuf, sizeof(recvBuf), &bytes, std::chrono::seconds(5))) << client.lastError();
    EXPECT_EQ(0, strncmp(recvBuf, ExpectedResponseLine, strlen(ExpectedResponseLine)));

    server.shutdown();
}

TEST(http_server_test, client_request_timeout_on_delay_in_header_send_raises_http_408)
{
    Pistache::Address address("localhost", Pistache::Port(0));

    const auto headerTimeout = std::chrono::seconds(1);

    Http::Endpoint server(address);
    auto flags = Tcp::Options::ReuseAddr;
    auto opts  = Http::Endpoint::options()
                    .flags(flags)
                    .headerTimeout(headerTimeout);

    server.init(opts);
    server.setHandler(Http::make_handler<PingHandler>());
    server.serveThreaded();

    auto port = server.getPort();
    auto addr = "localhost:" + port.toString();
    LOGGER("test", "Server address: " << addr);

    const std::string reqStr    = "GET /ping HTTP/1.1\r\n";
    const std::string headerStr = "Host: localhost\r\nUser-Agent: test\r\n";

    TcpClient client;
    EXPECT_TRUE(client.connect(Pistache::Address("localhost", port))) << client.lastError();
    EXPECT_TRUE(client.send(reqStr)) << client.lastError();

    std::this_thread::sleep_for(headerTimeout / 2);
    EXPECT_TRUE(client.send(headerStr)) << client.lastError();

    char recvBuf[1024] = {
        0,
    };
    size_t bytes;
    EXPECT_TRUE(client.receive(recvBuf, sizeof(recvBuf), &bytes, std::chrono::seconds(5))) << client.lastError();
    EXPECT_EQ(0, strncmp(recvBuf, ExpectedResponseLine, strlen(ExpectedResponseLine)));

    server.shutdown();
}

TEST(http_server_test, client_request_timeout_on_delay_in_request_line_send_raises_http_408)
{
    Pistache::Address address("localhost", Pistache::Port(0));

    const auto headerTimeout = std::chrono::seconds(2);

    Http::Endpoint server(address);
    auto flags = Tcp::Options::ReuseAddr;
    auto opts  = Http::Endpoint::options()
                    .flags(flags)
                    .headerTimeout(headerTimeout);

    server.init(opts);
    server.setHandler(Http::make_handler<PingHandler>());
    server.serveThreaded();

    auto port = server.getPort();
    auto addr = "localhost:" + port.toString();
    LOGGER("test", "Server address: " << addr);

    const std::string reqStr { "GET /ping HTTP/1.1\r\n" };
    TcpClient client;
    EXPECT_TRUE(client.connect(Pistache::Address("localhost", port))) << client.lastError();
    for (size_t i = 0; i < reqStr.size(); ++i)
    {
        if (!client.send(reqStr.substr(i, 1)))
        {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(300));
    }

    EXPECT_EQ(client.lastErrno(), EPIPE) << "Errno: " << client.lastErrno();

    char recvBuf[1024] = {
        0,
    };
    size_t bytes;
    EXPECT_TRUE(client.receive(recvBuf, sizeof(recvBuf), &bytes, std::chrono::seconds(5))) << client.lastError();
    EXPECT_EQ(0, strncmp(recvBuf, ExpectedResponseLine, strlen(ExpectedResponseLine)));

    server.shutdown();
}

TEST(http_server_test, client_request_timeout_on_delay_in_body_send_raises_http_408)
{
    Pistache::Address address("localhost", Pistache::Port(0));

    const auto headerTimeout = std::chrono::seconds(1);
    const auto bodyTimeout   = std::chrono::seconds(2);

    Http::Endpoint server(address);
    auto flags = Tcp::Options::ReuseAddr;
    auto opts  = Http::Endpoint::options()
                    .flags(flags)
                    .headerTimeout(headerTimeout)
                    .bodyTimeout(bodyTimeout);

    server.init(opts);
    server.setHandler(Http::make_handler<PingHandler>());
    server.serveThreaded();

    auto port = server.getPort();
    auto addr = "localhost:" + port.toString();
    LOGGER("test", "Server address: " << addr);

    const std::string reqStr = "POST /ping HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\nContent-Length: 32\r\n\r\nabc";

    TcpClient client;
    EXPECT_TRUE(client.connect(Pistache::Address("localhost", port))) << client.lastError();
    EXPECT_TRUE(client.send(reqStr)) << client.lastError();

    char recvBuf[1024] = {
        0,
    };
    size_t bytes;
    EXPECT_TRUE(client.receive(recvBuf, sizeof(recvBuf), &bytes, std::chrono::seconds(5))) << client.lastError();
    EXPECT_EQ(0, strncmp(recvBuf, ExpectedResponseLine, strlen(ExpectedResponseLine)));

    server.shutdown();
}

TEST(http_server_test, client_request_no_timeout)
{
    Pistache::Address address("localhost", Pistache::Port(0));

    const auto headerTimeout = std::chrono::seconds(2);
    const auto bodyTimeout   = std::chrono::seconds(4);

    Http::Endpoint server(address);
    auto flags = Tcp::Options::ReuseAddr;
    auto opts  = Http::Endpoint::options()
                    .flags(flags)
                    .headerTimeout(headerTimeout)
                    .bodyTimeout(bodyTimeout);

    server.init(opts);
    server.setHandler(Http::make_handler<PingHandler>());
    server.serveThreaded();

    auto port = server.getPort();
    auto addr = "localhost:" + port.toString();
    LOGGER("test", "Server address: " << addr);

    const std::string headerStr = "POST /ping HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\nContent-Length: 8\r\n\r\n";
    const std::string bodyStr   = "abcdefgh\r\n\r\n";

    TcpClient client;
    EXPECT_TRUE(client.connect(Pistache::Address("localhost", port))) << client.lastError();

    std::this_thread::sleep_for(headerTimeout / 2);
    EXPECT_TRUE(client.send(headerStr)) << client.lastError();

    std::this_thread::sleep_for(bodyTimeout / 2);
    EXPECT_TRUE(client.send(bodyStr)) << client.lastError();

    char recvBuf[1024] = {
        0,
    };
    size_t bytes;
    EXPECT_TRUE(client.receive(recvBuf, sizeof(recvBuf), &bytes, std::chrono::seconds(5))) << client.lastError();
    EXPECT_NE(0, strncmp(recvBuf, ExpectedResponseLine, strlen(ExpectedResponseLine)));

    server.shutdown();
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

    struct ClientCountingHandler : public Http::Handler
    {
        HTTP_PROTOTYPE(ClientCountingHandler)

        explicit ClientCountingHandler(std::shared_ptr<WaitHelper> waitHelper)
            : waitHelper(waitHelper)
        { }

        void onRequest(const Http::Request& request,
                       Http::ResponseWriter writer) override
        {
            auto peer = writer.getPeer();
            if (peer)
            {
                activeConnections.insert(peer->getID());
            }
            else
            {
                return;
            }
            std::string requestAddress = request.address().host();
            writer.send(Http::Code::Ok, requestAddress);
            LOGGER("server", "Sent `" << requestAddress << "` to " << *peer);
        }

        void onDisconnection(const std::shared_ptr<Tcp::Peer>& peer) override
        {
            LOGGER("server", "Disconnect from " << *peer);
            activeConnections.erase(peer->getID());
            waitHelper->increment();
        }

    private:
        std::unordered_set<size_t> activeConnections;
        std::shared_ptr<WaitHelper> waitHelper;
    };

} // namespace

TEST(http_server_test, client_multiple_requests_disconnects_handled)
{
    const Pistache::Address address("localhost", Pistache::Port(0));

    Http::Endpoint server(address);
    auto flags       = Tcp::Options::ReuseAddr;
    auto server_opts = Http::Endpoint::options().flags(flags);
    server.init(server_opts);

    std::cout << "Trying to run server...\n";
    auto waitHelper = std::make_shared<WaitHelper>();
    auto handler    = Http::make_handler<ClientCountingHandler>(waitHelper);
    server.setHandler(handler);
    server.serveThreaded();

    const std::string server_address = "localhost:" + server.getPort().toString();
    std::cout << "Server address: " << server_address << "\n";

    const size_t CLIENT_REQUEST_SIZE = 3;
    clientLogicFunc(CLIENT_REQUEST_SIZE, server_address, 1, 6);

    const bool result = waitHelper->wait(CLIENT_REQUEST_SIZE, std::chrono::seconds(2));
    server.shutdown();

    ASSERT_EQ(result, true);
}
