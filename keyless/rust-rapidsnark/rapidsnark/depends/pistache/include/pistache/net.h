/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* net.h
   Mathieu Stefani, 12 August 2015

   Network utility classes
*/

#pragma once

#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>

#include <netdb.h>
#include <netinet/in.h>
#include <sys/socket.h>

#ifndef _KERNEL_FASTOPEN
#define _KERNEL_FASTOPEN

/* conditional define for TCP_FASTOPEN */
#ifndef TCP_FASTOPEN
#define TCP_FASTOPEN 23
#endif
#endif

namespace Pistache
{

    // Wrapper around 'getaddrinfo()' that handles cleanup on destruction.
    class AddrInfo
    {
    public:
        // Disable copy and assign.
        AddrInfo(const AddrInfo&) = delete;
        AddrInfo& operator=(const AddrInfo&) = delete;

        // Default construction: do nothing.
        AddrInfo() = default;

        ~AddrInfo()
        {
            if (addrs)
            {
                ::freeaddrinfo(addrs);
            }
        }

        // Call "::getaddrinfo()", but stash result locally.  Takes the same args
        // as the first 3 args to "::getaddrinfo()" and returns the same result.
        int invoke(const char* node, const char* service,
                   const struct addrinfo* hints)
        {
            if (addrs)
            {
                ::freeaddrinfo(addrs);
                addrs = nullptr;
            }

            return ::getaddrinfo(node, service, hints, &addrs);
        }

        const struct addrinfo* get_info_ptr() const { return addrs; }

    private:
        struct addrinfo* addrs = nullptr;
    };

    class Port
    {
    public:
        Port(uint16_t port = 0);
        explicit Port(const std::string& data);

        operator uint16_t() const { return port; }

        bool isReserved() const;
        bool isUsed() const;
        std::string toString() const;

        static constexpr uint16_t min()
        {
            return std::numeric_limits<uint16_t>::min();
        }
        static constexpr uint16_t max()
        {
            return std::numeric_limits<uint16_t>::max();
        }

    private:
        uint16_t port;
    };

    class IP
    {
    private:
        int port;
        int family;
        union
        {
            struct sockaddr_in addr;
            struct sockaddr_in6 addr6;
        };

    public:
        IP();
        IP(uint8_t a, uint8_t b, uint8_t c, uint8_t d);
        IP(uint16_t a, uint16_t b, uint16_t c, uint16_t d, uint16_t e, uint16_t f,
           uint16_t g, uint16_t h);
        explicit IP(struct sockaddr*);
        static IP any();
        static IP loopback();
        static IP any(bool ipv6);
        static IP loopback(bool ipv6);
        int getFamily() const;
        int getPort() const;
        std::string toString() const;
        void toNetwork(in_addr_t*) const;
        void toNetwork(struct in6_addr*) const;
        // Returns 'true' if the system has IPV6 support, false if not.
        static bool supported();
    };
    using Ipv4 = IP;
    using Ipv6 = IP;

    class AddressParser
    {
    public:
        explicit AddressParser(const std::string& data);
        const std::string& rawHost() const;
        const std::string& rawPort() const;
        bool hasColon() const;
        int family() const;

    private:
        std::string host_;
        std::string port_;
        bool hasColon_ = false;
        int family_    = 0;
    };

    class Address
    {
    public:
        Address();
        Address(std::string host, Port port);
        explicit Address(std::string addr);
        explicit Address(const char* addr);
        Address(IP ip, Port port);

        Address(const Address& other) = default;
        Address(Address&& other)      = default;

        Address& operator=(const Address& other) = default;
        Address& operator=(Address&& other) = default;

        static Address fromUnix(struct sockaddr* addr);
        static Address fromUnix(struct sockaddr_in* addr);

        std::string host() const;
        Port port() const;
        int family() const;

        friend std::ostream& operator<<(std::ostream& os, const Address& address);

    private:
        void init(const std::string& addr);
        IP ip_;
        Port port_;
    };

    std::ostream& operator<<(std::ostream& os, const Address& address);

    namespace helpers
    {
        inline Address httpAddr(const std::string_view& view)
        {
            return Address(std::string(view));
        }
    } // namespace helpers

    class Error : public std::runtime_error
    {
    public:
        explicit Error(const char* message);
        explicit Error(std::string message);
        static Error system(const char* message);
    };

    template <typename T>
    struct Size
    { };

    template <typename T>
    size_t digitsCount(T val)
    {
        size_t digits = 0;
        while (val % 10)
        {
            ++digits;

            val /= 10;
        }

        return digits;
    }

    template <>
    struct Size<const char*>
    {
        size_t operator()(const char* s) const { return std::strlen(s); }
    };

    template <size_t N>
    struct Size<char[N]>
    {
        constexpr size_t operator()(const char (&)[N]) const
        {

            // We omit the \0
            return N - 1;
        }
    };

#define DEFINE_INTEGRAL_SIZE(Int)                                     \
    template <>                                                       \
    struct Size<Int>                                                  \
    {                                                                 \
        size_t operator()(Int val) const { return digitsCount(val); } \
    }

    DEFINE_INTEGRAL_SIZE(uint8_t);
    DEFINE_INTEGRAL_SIZE(int8_t);
    DEFINE_INTEGRAL_SIZE(uint16_t);
    DEFINE_INTEGRAL_SIZE(int16_t);
    DEFINE_INTEGRAL_SIZE(uint32_t);
    DEFINE_INTEGRAL_SIZE(int32_t);
    DEFINE_INTEGRAL_SIZE(uint64_t);
    DEFINE_INTEGRAL_SIZE(int64_t);

    template <>
    struct Size<bool>
    {
        constexpr size_t operator()(bool) const { return 1; }
    };

    template <>
    struct Size<char>
    {
        constexpr size_t operator()(char) const { return 1; }
    };

} // namespace Pistache
