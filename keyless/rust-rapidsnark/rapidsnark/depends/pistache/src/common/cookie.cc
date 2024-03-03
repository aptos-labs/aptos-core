/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 16 janvier 2016

   Cookie implementation
*/

#include <pistache/config.h>
#include <pistache/cookie.h>
#include <pistache/stream.h>

#include <iterator>
#include <optional>
#include <unordered_map>

namespace Pistache::Http
{

    namespace
    {

        StreamCursor::Token matchValue(StreamCursor& cursor)
        {
            int c;
            if ((c = cursor.current()) != StreamCursor::Eof && c != '=')
                throw std::runtime_error("Invalid cookie");

            if (!cursor.advance(1))
                throw std::runtime_error("Invalid cookie, early eof");

            StreamCursor::Token token(cursor);
            match_until(';', cursor);

            return token;
        }

        template <typename T>
        struct AttributeMatcher;

        template <>
        struct AttributeMatcher<std::optional<std::string>>
        {
            static void match(StreamCursor& cursor, Cookie* obj,
                              std::optional<std::string> Cookie::*attr)
            {
                auto token = matchValue(cursor);
                obj->*attr = token.text();
            }
        };

        template <>
        struct AttributeMatcher<std::optional<int>>
        {
            static void match(StreamCursor& cursor, Cookie* obj,
                              std::optional<int> Cookie::*attr)
            {
                auto token = matchValue(cursor);

                auto strntol = [](const char* str, size_t len) {
                    int ret = 0;
                    for (size_t i = 0; i < len; ++i)
                    {
                        if (!isdigit(str[i]))
                            throw std::invalid_argument("Invalid conversion");

                        ret *= 10;
                        ret += str[i] - '0';
                    };

                    return ret;
                };

                obj->*attr = strntol(token.rawText(), token.size());
            }
        };

        template <>
        struct AttributeMatcher<bool>
        {
            static void match(StreamCursor& /*cursor*/, Cookie* obj, bool Cookie::*attr)
            {
                obj->*attr = true;
            }
        };

        template <>
        struct AttributeMatcher<std::optional<FullDate>>
        {
            static void match(StreamCursor& cursor, Cookie* obj,
                              std::optional<FullDate> Cookie::*attr)
            {
                auto token = matchValue(cursor);
                obj->*attr = FullDate::fromString(token.text());
            }
        };

        template <typename T>
        bool match_attribute(const char* name, size_t len, StreamCursor& cursor,
                             Cookie* obj, T Cookie::*attr)
        {
            if (match_string(name, len, cursor))
            {
                AttributeMatcher<T>::match(cursor, obj, attr);
                cursor.advance(1);

                return true;
            }

            return false;
        }

    } // namespace

    Cookie::Cookie(std::string name, std::string value)
        : name(std::move(name))
        , value(std::move(value))
        , path()
        , domain()
        , expires()
        , maxAge()
        , secure(false)
        , httpOnly(false)
        , ext()
    { }

    Cookie Cookie::fromRaw(const char* str, size_t len)
    {
        RawStreamBuf<> buf(const_cast<char*>(str), len);
        StreamCursor cursor(&buf);

        StreamCursor::Token nameToken(cursor);

        if (!match_until('=', cursor))
            throw std::runtime_error("Invalid cookie, missing value");

        auto name_ = nameToken.text();

        if (!cursor.advance(1))
            throw std::runtime_error("Invalid cookie, missing value");

        StreamCursor::Token valueToken(cursor);

        match_until(';', cursor);
        auto value_ = valueToken.text();

        Cookie cookie(std::move(name_), std::move(value_));
        if (cursor.eof())
        {
            return cookie;
        }

        cursor.advance(1);

#define STR(str) str, sizeof(str) - 1

        do
        {
            skip_whitespaces(cursor);

            if (match_attribute(STR("Path"), cursor, &cookie, &Cookie::path))
                ;
            else if (match_attribute(STR("Domain"), cursor, &cookie, &Cookie::domain))
                ;
            else if (match_attribute(STR("Secure"), cursor, &cookie, &Cookie::secure))
                ;
            else if (match_attribute(STR("HttpOnly"), cursor, &cookie,
                                     &Cookie::httpOnly))
                ;
            else if (match_attribute(STR("Max-Age"), cursor, &cookie, &Cookie::maxAge))
                ;
            else if (match_attribute(STR("Expires"), cursor, &cookie, &Cookie::expires))
                ;
            // ext
            else
            {
                StreamCursor::Token nameToken_(cursor);
                match_until('=', cursor);

                auto name = nameToken_.text();
                std::string value;
                if (!cursor.eof())
                {
                    auto token = matchValue(cursor);
                    value      = token.text();
                }
                cookie.ext.insert(std::make_pair(std::move(name), std::move(value)));
            }

        } while (!cursor.eof());

#undef STR

        return cookie;
    }

    Cookie Cookie::fromString(const std::string& str)
    {
        return Cookie::fromRaw(str.c_str(), str.size());
    }

    void Cookie::write(std::ostream& os) const
    {
        os << name << "=" << value;
        if (path.has_value())
        {
            const std::string& value = *path;
            os << "; ";
            os << "Path=" << value;
        }
        if (domain.has_value())
        {
            const std::string& value = *domain;
            os << "; ";
            os << "Domain=" << value;
        }
        if (maxAge.has_value())
        {
            int value = *maxAge;
            os << "; ";
            os << "Max-Age=" << value;
        }
        if (expires.has_value())
        {
            const FullDate& value = *expires;
            os << "; ";
            os << "Expires=";
            value.write(os);
        }
        if (secure)
            os << "; Secure";
        if (httpOnly)
            os << "; HttpOnly";
        if (!ext.empty())
        {
            os << "; ";
            for (auto it = std::begin(ext), end = std::end(ext); it != end; ++it)
            {
                os << it->first << "=" << it->second;
                if (std::distance(it, end) > 1)
                    os << "; ";
            }
        }
    }

    std::ostream& operator<<(std::ostream& os, const Cookie& cookie)
    {
        cookie.write(os);
        return os;
    }

    CookieJar::CookieJar()
        : cookies()
    { }

    void CookieJar::add(const Cookie& cookie)
    {

        std::string cookieName  = cookie.name;
        std::string cookieValue = cookie.value;

        Storage::iterator it = cookies.find(cookieName);
        if (it == cookies.end())
        {
            HashMapCookies hashmapWithFirstCookie;
            hashmapWithFirstCookie.insert(std::make_pair(cookieValue, cookie));
            cookies.insert(std::make_pair(cookieName, hashmapWithFirstCookie));
        }
        else
        {
            it->second.insert(std::make_pair(cookieValue, cookie));
        }
    }

    void CookieJar::removeAllCookies() { cookies.clear(); }

    void CookieJar::addFromRaw(const char* str, size_t len)
    {
        RawStreamBuf<> buf(const_cast<char*>(str), len);
        StreamCursor cursor(&buf);

        while (!cursor.eof())
        {
            StreamCursor::Token nameToken(cursor);

            if (!match_until('=', cursor))
                throw std::runtime_error("Invalid cookie, missing value");

            auto name = nameToken.text();

            if (!cursor.advance(1))
                throw std::runtime_error("Invalid cookie, missing value");

            StreamCursor::Token valueToken(cursor);

            match_until(';', cursor);
            auto value = valueToken.text();

            Cookie cookie(std::move(name), std::move(value));
            add(cookie);

            cursor.advance(1);
            skip_whitespaces(cursor);
        }
    }

    Cookie CookieJar::get(const std::string& name) const
    {
        Storage::const_iterator it = cookies.find(name);
        if (it != cookies.end())
        {
            return it->second.begin()
                ->second; // it returns begin(), first element, could be changed.
        }
        throw std::runtime_error("Could not find requested cookie");
    }

    bool CookieJar::has(const std::string& name) const
    {
        return cookies.find(name) != cookies.end();
    }

} // namespace Pistache::Http
