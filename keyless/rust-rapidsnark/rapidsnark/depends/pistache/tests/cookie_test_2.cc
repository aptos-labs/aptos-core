/*
 * SPDX-FileCopyrightText: 2018 hasankandemir1993
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>

#include <date/date.h>
#include <pistache/cookie.h>

using namespace Pistache;
using namespace Pistache::Http;

void addCookies(const char* str,
                std::function<void(const CookieJar&)> testFunc)
{
    CookieJar jar;
    jar.addFromRaw(str, strlen(str));
    testFunc(jar);
}

TEST(cookie_test_2, cookiejar_test_2)
{
    addCookies("key=value1; key=value2; key2=; key2=foo=bar",
               [](const CookieJar& jar) {
                   int count = 0;

                   for (const auto& c : jar)
                   {
                       static_cast<void>(c); // 'c' is unused
                       count++;
                   }

                   ASSERT_EQ(count, 4); // number of cookies must be 4 in this case
               });
}

TEST(cookie_test_2, cookiejar_iterator)
{
    // NOTE: Cookies are stored in an unordered map.  Iterator order
    // is NOT guaranteed (infact, will be different on different versions
    // of libc++).

    std::unordered_map<std::string, std::string> control = {
        { "a", "blossom" },
        { "b", "bubbles" },
        { "c", "buttercup" },
    };

    addCookies("a=blossom; b=bubbles; c=buttercup",
               [&control](const CookieJar& jar) {
                   auto i = jar.begin();

                   // Test "operator*"
                   do
                   {
                       const auto name  = (*i).name;
                       const auto value = control.at(name);
                       ASSERT_EQ((*i).value, value);
                   } while (false);

                   // Test "operator->" and pre-increment++
                   do
                   {
                       const auto r     = ++i;
                       const auto name  = i->name;
                       const auto value = control.at(name);
                       ASSERT_EQ(i->value, value);
                       ASSERT_EQ(r->name, i->name);
                   } while (false);

                   // Test "operator->" and post-increment++
                   do
                   {
                       const auto r     = i++;
                       const auto name  = i->name;
                       const auto value = control.at(name);
                       ASSERT_EQ(i->value, value);
                       ASSERT_NE(r->name, i->name);
                   } while (false);

                   // pre-increment should end the iterator.
                   ++i;
                   ASSERT_EQ(i, jar.end());
               });
}
