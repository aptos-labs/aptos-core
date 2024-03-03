/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* http_headers.h
   Mathieu Stefani, 19 August 2015

   A list of HTTP headers
*/

#pragma once

#include <algorithm>
#include <functional>
#include <memory>
#include <unordered_map>
#include <vector>

#include <pistache/http_header.h>
#include <pistache/type_checkers.h>

namespace Pistache::Http::Header
{

    std::string toLowercase(std::string str);

    struct LowercaseHash
    {
        size_t operator()(const std::string& key) const
        {
            return std::hash<std::string> {}(toLowercase(key));
        }
    };

    bool LowercaseEqualStatic(const std::string& dynamic,
                              const std::string& statik);

    struct LowercaseEqual
    {
        bool operator()(const std::string& left, const std::string& right) const
        {
            return std::equal(left.begin(), left.end(), right.begin(), right.end(),
                              [](const char& a, const char& b) {
                                  return std::tolower(a) == std::tolower(b);
                              });
        };
    };

    class Collection
    {
    public:
        Collection()
            : headers()
            , rawHeaders()
        { }

        template <typename H>
        typename std::enable_if<IsHeader<H>::value, std::shared_ptr<const H>>::type
        get() const
        {
            return std::static_pointer_cast<const H>(get(H::Name));
        }
        template <typename H>
        typename std::enable_if<IsHeader<H>::value, std::shared_ptr<H>>::type get()
        {
            return std::static_pointer_cast<H>(get(H::Name));
        }

        template <typename H>
        typename std::enable_if<IsHeader<H>::value, std::shared_ptr<const H>>::type
        tryGet() const
        {
            return std::static_pointer_cast<const H>(tryGet(H::Name));
        }
        template <typename H>
        typename std::enable_if<IsHeader<H>::value, std::shared_ptr<H>>::type
        tryGet()
        {
            return std::static_pointer_cast<H>(tryGet(H::Name));
        }

        Collection& add(const std::shared_ptr<Header>& header);
        Collection& addRaw(const Raw& raw);

        template <typename H, typename... Args>
        typename std::enable_if<IsHeader<H>::value, Collection&>::type
        add(Args&&... args)
        {
            return add(std::make_shared<H>(std::forward<Args>(args)...));
        }

        template <typename H>
        typename std::enable_if<IsHeader<H>::value, bool>::type remove()
        {
            return remove(H::Name);
        }

        std::shared_ptr<const Header> get(const std::string& name) const;
        std::shared_ptr<Header> get(const std::string& name);
        Raw getRaw(const std::string& name) const;

        std::shared_ptr<const Header> tryGet(const std::string& name) const;
        std::shared_ptr<Header> tryGet(const std::string& name);
        std::optional<Raw> tryGetRaw(const std::string& name) const;

        template <typename H>
        typename std::enable_if<IsHeader<H>::value, bool>::type has() const
        {
            return has(H::Name);
        }
        bool has(const std::string& name) const;

        std::vector<std::shared_ptr<Header>> list() const;

        const std::unordered_map<std::string, Raw, LowercaseHash, LowercaseEqual>&
        rawList() const
        {
            return rawHeaders;
        }

        bool remove(const std::string& name);

        void clear();

    private:
        std::pair<bool, std::shared_ptr<Header>>
        getImpl(const std::string& name) const;

        std::unordered_map<std::string, std::shared_ptr<Header>, LowercaseHash,
                           LowercaseEqual>
            headers;
        std::unordered_map<std::string, Raw, LowercaseHash, LowercaseEqual>
            rawHeaders;
    };

    class Registry
    {

    public:
        Registry(const Registry&) = delete;
        Registry& operator=(const Registry&) = delete;
        static Registry& instance();

        template <typename H, REQUIRES(IsHeader<H>::value)>
        void registerHeader()
        {
            registerHeader(H::Name, []() -> std::unique_ptr<Header> {
                return std::unique_ptr<Header>(new H());
            });
        }

        std::vector<std::string> headersList();

        std::unique_ptr<Header> makeHeader(const std::string& name);
        bool isRegistered(const std::string& name);

    private:
        Registry();
        ~Registry();

        using RegistryFunc        = std::function<std::unique_ptr<Header>()>;
        using RegistryStorageType = std::unordered_map<std::string, RegistryFunc,
                                                       LowercaseHash, LowercaseEqual>;

        void registerHeader(const std::string& name, RegistryFunc func);

        RegistryStorageType registry;
    };

    template <typename H>
    struct Registrar
    {
        static_assert(IsHeader<H>::value, "Registrar only works with header types");

        Registrar() { Registry::instance().registerHeader<H>(); }
    };

/* Crazy macro machinery to generate a unique variable name
 * Don't touch it !
 */
#define CAT(a, b) CAT_I(a, b)
#define CAT_I(a, b) a##b

#define UNIQUE_NAME(base) CAT(base, __LINE__)

#define RegisterHeader(Header) \
    Registrar<Header> UNIQUE_NAME(CAT(CAT_I(__, Header), __))

} // namespace Pistache::Http::Header
