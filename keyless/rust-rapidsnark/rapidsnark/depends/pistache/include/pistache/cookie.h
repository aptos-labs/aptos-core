/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 16 janvier 2016

   Representation of a Cookie as per http://tools.ietf.org/html/rfc6265
*/

#pragma once

#include <cstddef>
#include <ctime>
#include <list>
#include <map>
#include <optional>
#include <string>
#include <unordered_map>
#include <unordered_set>

#include <pistache/http_defs.h>

namespace Pistache::Http
{

    struct Cookie
    {
        friend std::ostream& operator<<(std::ostream& os, const Cookie& cookie);

        Cookie(std::string name, std::string value);

        std::string name;
        std::string value;

        std::optional<std::string> path;
        std::optional<std::string> domain;
        std::optional<FullDate> expires;

        std::optional<int> maxAge;
        bool secure;
        bool httpOnly;

        std::map<std::string, std::string> ext;

        static Cookie fromRaw(const char* str, size_t len);
        static Cookie fromString(const std::string& str);

    private:
        void write(std::ostream& os) const;
    };

    std::ostream& operator<<(std::ostream& os, const Cookie& cookie);

    class CookieJar
    {
    public:
        using HashMapCookies = std::unordered_map<std::string, Cookie>; // "value" -> Cookie
        using Storage        = std::unordered_map<
            std::string, HashMapCookies>; // "name" -> Hashmap("value" -> Cookie)

        struct iterator
        {
            using iterator_category = std::bidirectional_iterator_tag;
            using value_type = Cookie;
            using difference_type = std::ptrdiff_t;
            using pointer = value_type*;
            using reference = value_type&;

            explicit iterator(const Storage::const_iterator& _iterator)
                : iter_storage(_iterator)
                , iter_cookie_values()
                , iter_storage_end()
            { }

            iterator(const Storage::const_iterator& _iterator,
                     const Storage::const_iterator& end)
                : iter_storage(_iterator)
                , iter_cookie_values()
                , iter_storage_end(end)
            {
                if (iter_storage != iter_storage_end)
                {
                    iter_cookie_values = iter_storage->second.begin();
                }
            }

            Cookie operator*() const
            {
                return iter_cookie_values->second; // return iter_storage->second;
            }

            const Cookie* operator->() const { return &(iter_cookie_values->second); }

            iterator& operator++()
            {
                ++iter_cookie_values;
                if (iter_cookie_values == iter_storage->second.end())
                {
                    ++iter_storage;
                    if (iter_storage != iter_storage_end)
                        iter_cookie_values = iter_storage->second.begin();
                }
                return *this;
            }

            iterator operator++(int)
            {
                iterator ret(iter_storage, iter_storage_end);
                ++iter_cookie_values;
                if (iter_cookie_values == iter_storage->second.end())
                {
                    ++iter_storage;
                    if (iter_storage != iter_storage_end) // this check is important
                        iter_cookie_values = iter_storage->second.begin();
                }

                return ret;
            }

            bool operator!=(iterator other) const
            {
                return iter_storage != other.iter_storage;
            }

            bool operator==(iterator other) const
            {
                return iter_storage == other.iter_storage;
            }

        private:
            Storage::const_iterator iter_storage;
            HashMapCookies::const_iterator iter_cookie_values;
            Storage::const_iterator
                iter_storage_end; // we need to know where main hashmap ends.
        };

        CookieJar();

        void add(const Cookie& cookie);
        void removeAllCookies();

        void addFromRaw(const char* str, size_t len);
        Cookie get(const std::string& name) const;

        bool has(const std::string& name) const;

        iterator begin() const { return iterator(cookies.begin(), cookies.end()); }

        iterator end() const { return iterator(cookies.end()); }

    private:
        Storage cookies;
    };

} // namespace Pistache::Http
