/*
 * SPDX-FileCopyrightText: 2018 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>
#include <pistache/http.h>
#include <pistache/stream.h>

#include <string>
#include <tuple>
#include <vector>

using namespace Pistache;

// @Todo: Add an easy to use fixture to inject data for parsing tests.

TEST(http_parsing_test, should_parse_http_request_in_two_packets_issue_160)
{
    Http::RequestParser parser(Const::DefaultMaxRequestSize);

    auto feed = [&parser](const char* data) {
        parser.feed(data, std::strlen(data));
    };

    // First, we feed the parser with a Request-Line
    feed("GET /hello HTTP/1.1\r\n");
    ASSERT_EQ(parser.parse(), Http::Private::State::Again);
    // @Todo @Completeness We should also assert that we are in the correct step.
    // However, the step is currently not exposed by the parser. Since the parser
    // is supposed to stay "private", we could either directly expose the step or
    // return it from the parse() method.

    // Let's now put some headers
    feed("User-Agent: Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, "
         "like Gecko) Chrome/41.0.2228.0 Safari/537.36\r\n");
    feed("Host: localhost\r\n");
    feed("Content-Length: 5\r\n");
    feed("\r\n");
    ASSERT_EQ(parser.parse(), Http::Private::State::Again);

    // Finally, we finish the body
    feed("HELLO");
    ASSERT_EQ(parser.parse(), Http::Private::State::Done);
}

TEST(http_parsing_test, parser_reset)
{
    Http::RequestParser parser(Const::DefaultMaxRequestSize);

    auto feed = [&parser](const char* data) {
        parser.feed(data, std::strlen(data));
    };

    feed("GET /hello?parameter=value HTTP/1.1\r\n");
    feed("User-Agent: Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, "
         "like Gecko) Chrome/41.0.2228.0 Safari/537.36\r\n");
    feed("Host: localhost\r\n");
    feed("Content-Length: 5\r\n");
    feed("\r\n");
    feed("HELLO");

    ASSERT_EQ(parser.parse(), Http::Private::State::Done);

    ASSERT_EQ(parser.request.query().as_str(), "?parameter=value");
    ASSERT_EQ(parser.request.resource(), "/hello");
    ASSERT_EQ(parser.request.headers().list().size(), 3u);
    ASSERT_EQ(parser.request.body(), "HELLO");

    parser.reset();

    ASSERT_EQ(parser.request.query().as_str(), "");
    ASSERT_EQ(parser.request.resource(), "");
    ASSERT_EQ(parser.request.headers().list().size(), 0u);
    ASSERT_EQ(parser.request.body(), "");
}

TEST(http_parsing_test, succ_response_line_step)
{
    Http::Response response;
    Http::Private::ResponseLineStep step(&response);

    std::string line("HTTP/1.1 200 OK\r\n");
    RawStreamBuf<> buf(&line[0], line.size());
    StreamCursor cursor(&buf);

    Http::Private::State state = step.apply(cursor);

    ASSERT_EQ(state, Http::Private::State::Next);
    ASSERT_EQ(response.code(), Http::Code::Ok);
    ASSERT_EQ(response.version(), Http::Version::Http11);
}

TEST(http_parsing_test, error_response_line_step)
{
    std::vector<std::string> lines = {
        "HTTP/ABC.DEF 200 OK\r\n", "HTTP/1.1200 OK\r\n", "HTTP/ABC.DEF 200\r\n"
    };
    for (auto& line : lines)
    {
        Http::Response response;
        Http::Private::ResponseLineStep step(&response);

        RawStreamBuf<> buf(&line[0], line.size());
        StreamCursor cursor(&buf);

        ASSERT_THROW(step.apply(cursor), Http::HttpError);
    }
}

TEST(http_parsing_test, again_response_line_step)
{
    std::vector<std::string> lines = { "HTTP/1.1 200 OK\r", "HTTP/1.1 200 OK" };
    for (auto& line : lines)
    {
        Http::Response response;
        Http::Private::ResponseLineStep step(&response);

        RawStreamBuf<> buf(&line[0], line.size());
        StreamCursor cursor(&buf);

        Http::Private::State state = step.apply(cursor);

        ASSERT_EQ(state, Http::Private::State::Again);
    }
}

TEST(http_parsing_test, succ_request_line_step)
{
    Http::Request request;
    Http::Private::RequestLineStep step(&request);

    std::string line("GET example.com HTTP/1.1\r\n");
    RawStreamBuf<> buf(&line[0], line.size());
    StreamCursor cursor(&buf);

    Http::Private::State state = step.apply(cursor);

    ASSERT_EQ(state, Http::Private::State::Next);
    ASSERT_EQ(request.method(), Http::Method::Get);
    ASSERT_EQ(request.resource(), "example.com");
    ASSERT_EQ(request.version(), Http::Version::Http11);
}

TEST(http_parsing_test, error_request_line_step)
{
    std::vector<std::string> lines = { "FOO example.com HTTP/1.1\r\n",
                                       "BAR example.com HTTP/1.1\r\n" };
    for (auto& line : lines)
    {
        Http::Request request;
        Http::Private::RequestLineStep step(&request);

        RawStreamBuf<> buf(&line[0], line.size());
        StreamCursor cursor(&buf);

        ASSERT_THROW(step.apply(cursor), Http::HttpError);
    }
}

TEST(http_parsing_test, again_request_line_step)
{
    std::vector<std::string> lines = { "", "FOO" };
    for (auto& line : lines)
    {
        Http::Request request;
        Http::Private::RequestLineStep step(&request);

        RawStreamBuf<> buf(&line[0], line.size());
        StreamCursor cursor(&buf);

        Http::Private::State state = step.apply(cursor);

        ASSERT_EQ(state, Http::Private::State::Again);
    }
}

TEST(http_parsing_test, succ_method_parse_request)
{
    enum class ResultType { Pass,
                            Fail };

    using TestDataHolder = std::vector<std::tuple<std::string, Http::Method, ResultType>>;
    TestDataHolder holder;
    holder.emplace_back("GET /resource HTTP/1.1\r\n", Http::Method::Get,
                        ResultType::Pass);
    holder.emplace_back("HEAD /resources HTTP/1.1\r\n", Http::Method::Head,
                        ResultType::Pass);
    holder.emplace_back("POST /resources HTTP/1.1\r\n", Http::Method::Post,
                        ResultType::Pass);
    holder.emplace_back("DELETE /resources HTTP/1.1\r\n", Http::Method::Delete,
                        ResultType::Pass);
    holder.emplace_back("REVERT /resources HTTP/1.1\r\n", Http::Method::Put,
                        ResultType::Fail);

    for (auto& item : holder)
    {
        Http::Request request;
        Http::Private::RequestLineStep step(&request);

        std::string reqestString = std::get<0>(item);
        RawStreamBuf<> buf(&reqestString[0], reqestString.size());
        StreamCursor cursor(&buf);

        if (std::get<2>(item) == ResultType::Pass)
        {
            Http::Private::State state = step.apply(cursor);
            ASSERT_EQ(state, Http::Private::State::Next);
            ASSERT_EQ(request.method(), std::get<1>(item));
        }
        else
        {
            ASSERT_THROW(step.apply(cursor), Http::HttpError);
        }
    }
}
