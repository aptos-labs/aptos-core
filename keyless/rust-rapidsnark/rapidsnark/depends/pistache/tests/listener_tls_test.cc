/*
 * SPDX-FileCopyrightText: 2023 Andrea Pappacoda
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gmock/gmock-matchers.h>
#include <gtest/gtest.h>

#include <pistache/listener.h>

#include <chrono>
#include <openssl/bio.h>
#include <pistache/http.h>

using testing::Eq;
using testing::Le;

class HelloHandler : public Pistache::Http::Handler
{
public:
    HTTP_PROTOTYPE(HelloHandler)

    void onRequest(const Pistache::Http::Request& /*request*/,
                   Pistache::Http::ResponseWriter response) override
    {
        response.send(Pistache::Http::Code::Ok, "Hello world\n");
    }
};

TEST(listener_tls_test, tls_handshake_timeout)
{
    Pistache::Tcp::Listener listener;
    listener.init(1);
    listener.setupSSL("./certs/server.crt", "./certs/server.key", false, nullptr);
    listener.setHandler(Pistache::Http::make_handler<HelloHandler>());
    listener.bind(Pistache::Address(Pistache::IP::loopback(), 0));
    listener.runThreaded();

    // Tell the BIO how to connect to the listener
    BIO* bio = BIO_new_connect("localhost");
    BIO_set_conn_port(bio, listener.getPort().toString().c_str());

    const auto pre_handshake = std::chrono::steady_clock::now();

    // Connect to the listener without actually initiating a TLS handshake
    long success = BIO_do_connect(bio);
    ASSERT_THAT(success, Eq(1));

    // Try to read something until the listener drops the connection. The
    // read is expected to fail
    unsigned char buf[10];
    success = BIO_read(bio, buf, sizeof buf);
    EXPECT_THAT(success, Le(0));

    const auto duration = std::chrono::steady_clock::now() - pre_handshake;

    // The timeout shouldn't be longer than 20 seconds by default
    EXPECT_THAT(duration, Le(std::chrono::seconds(20)));

    BIO_free_all(bio);
}

TEST(listener_tls_test, tls_handshake_timeout_custom)
{
    Pistache::Tcp::Listener listener;
    listener.init(1);
    listener.setupSSL("./certs/server.crt", "./certs/server.key", false, nullptr, std::chrono::seconds(3));
    listener.setHandler(Pistache::Http::make_handler<HelloHandler>());
    listener.bind(Pistache::Address(Pistache::IP::loopback(), 0));
    listener.runThreaded();

    // Tell the BIO how to connect to the listener
    BIO* bio = BIO_new_connect("localhost");
    BIO_set_conn_port(bio, listener.getPort().toString().c_str());

    const auto pre_handshake = std::chrono::steady_clock::now();

    // Connect to the listener without actually initiating a TLS handshake
    long success = BIO_do_connect(bio);
    ASSERT_THAT(success, Eq(1));

    // Try to read something until the listener drops the connection. The
    // read is expected to fail
    unsigned char buf[10];
    success = BIO_read(bio, buf, sizeof buf);
    EXPECT_THAT(success, Le(0));

    const auto duration = std::chrono::steady_clock::now() - pre_handshake;

    // The timeout shouldn't be longer than 5 seconds
    EXPECT_THAT(duration, Le(std::chrono::seconds(5)));

    BIO_free_all(bio);
}
