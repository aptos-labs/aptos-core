/*
 * SPDX-FileCopyrightText: 2021 bbarbisch
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/endpoint.h>
#include <pistache/router.h>

#include <gtest/gtest.h>

using namespace Pistache;

TEST(endpoint_initialization_test, initialize_options_before_handler)
{
    Rest::Router router;
    auto handler = router.handler();
    Address addr(Ipv4::any(), Port(0));
    Http::Endpoint endpoint(addr);

    size_t maxReqSize  = 123;
    size_t maxRespSize = 456;

    auto opts = Http::Endpoint::options();
    opts.threads(2);
    opts.maxRequestSize(maxReqSize);
    opts.maxResponseSize(maxRespSize);

    endpoint.init(opts);
    endpoint.setHandler(handler);

    ASSERT_EQ(handler->getMaxRequestSize(), maxReqSize);
    ASSERT_EQ(handler->getMaxResponseSize(), maxRespSize);
}

TEST(endpoint_initialization_test, initialize_handler_before_options)
{
    Rest::Router router;
    auto handler = router.handler();
    Address addr(Ipv4::any(), Port(0));
    Http::Endpoint endpoint(addr);

    size_t maxReqSize  = 123;
    size_t maxRespSize = 456;

    auto opts = Http::Endpoint::options();
    opts.threads(2);
    opts.maxRequestSize(maxReqSize);
    opts.maxResponseSize(maxRespSize);

    endpoint.setHandler(handler);
    endpoint.init(opts);

    ASSERT_EQ(handler->getMaxRequestSize(), maxReqSize);
    ASSERT_EQ(handler->getMaxResponseSize(), maxRespSize);
}
