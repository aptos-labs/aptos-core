/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>
#include <pistache/http.h>

using namespace Pistache;

TEST(http_uri_test, query_as_string_test)
{
    Http::Uri::Query query1;
    ASSERT_TRUE(query1.as_str().empty());

    Http::Uri::Query query2;
    query2.add("value1", "name1");
    ASSERT_STREQ(query2.as_str().c_str(), "?value1=name1");

    Http::Uri::Query query3;
    query3.add("value1", "name1");
    query3.add("value2", "name2");
    ASSERT_STREQ(query3.as_str().c_str(), "?value2=name2&value1=name1");
}