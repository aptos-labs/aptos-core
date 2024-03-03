/*
 * SPDX-FileCopyrightText: 2018 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>

#include <pistache/net.h>

#include <iostream>
#include <stdexcept>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>

using namespace Pistache;

TEST(net_test, port_creation)
{
    Port port1(3000);
    ASSERT_FALSE(port1.isReserved());
    uint16_t value1 = port1;
    ASSERT_EQ(value1, 3000);
    ASSERT_EQ(port1.toString(), "3000");

    Port port2(80);
    ASSERT_TRUE(port2.isReserved());
    uint16_t value2 = port2;
    ASSERT_EQ(value2, 80);
    ASSERT_EQ(port2.toString(), "80");
}

TEST(net_test, address_creation)
{
    Address address1("127.0.0.1:8080");
    ASSERT_EQ(address1.host(), "127.0.0.1");
    ASSERT_EQ(address1.family(), AF_INET);
    ASSERT_EQ(address1.port(), 8080);

    std::string addr = "127.0.0.1";
    Address address2(addr, Port(8080));
    ASSERT_EQ(address2.host(), "127.0.0.1");
    ASSERT_EQ(address2.family(), AF_INET);
    ASSERT_EQ(address2.port(), 8080);

    Address address3(Ipv4(127, 0, 0, 1), Port(8080));
    ASSERT_EQ(address3.host(), "127.0.0.1");
    ASSERT_EQ(address3.family(), AF_INET);
    ASSERT_EQ(address3.port(), 8080);

    Address address4(Ipv4::any(), Port(8080));
    ASSERT_EQ(address4.host(), "0.0.0.0");
    ASSERT_EQ(address4.family(), AF_INET);
    ASSERT_EQ(address4.port(), 8080);

    Address address5("*:8080");
    ASSERT_EQ(address5.host(), "0.0.0.0");
    ASSERT_EQ(address5.family(), AF_INET);
    ASSERT_EQ(address5.port(), 8080);

    Address address6("[::1]:8080");
    ASSERT_EQ(address6.host(), "::1");
    ASSERT_EQ(address6.family(), AF_INET6);
    ASSERT_EQ(address6.port(), 8080);

    std::string addr2 = "[::1]";
    Address address7(addr2, Port(8080));
    ASSERT_EQ(address7.host(), "::1");
    ASSERT_EQ(address7.family(), AF_INET6);
    ASSERT_EQ(address7.port(), 8080);

    Address address8(Ipv6(0, 0, 0, 0, 0, 0, 0, 1), Port(8080));
    ASSERT_EQ(address8.host(), "::1");
    ASSERT_EQ(address8.family(), AF_INET6);
    ASSERT_EQ(address8.port(), 8080);

    Address address9(Ipv6::any(true), Port(8080));
    ASSERT_EQ(address9.host(), "::");
    ASSERT_EQ(address9.family(), AF_INET6);
    ASSERT_EQ(address9.port(), 8080);

    Address address10("[::]:8080");
    ASSERT_EQ(address10.host(), "::");
    ASSERT_EQ(address10.family(), AF_INET6);
    ASSERT_EQ(address10.port(), 8080);

    Address address11("[2001:0DB8:AABB:CCDD:EEFF:0011:2233:4455]:8080");
    ASSERT_EQ(address11.host(), "2001:db8:aabb:ccdd:eeff:11:2233:4455");
    ASSERT_EQ(address11.family(), AF_INET6);
    ASSERT_EQ(address11.port(), 8080);

    Address address12(Ipv4::loopback(), Port(8080));
    ASSERT_EQ(address12.host(), "127.0.0.1");
    ASSERT_EQ(address12.family(), AF_INET);
    ASSERT_EQ(address12.port(), 8080);

    Address address13(Ipv6::loopback(true), Port(8080));
    ASSERT_EQ(address13.host(), "::1");
    ASSERT_EQ(address13.family(), AF_INET6);
    ASSERT_EQ(address13.port(), 8080);

    Address address14("127.0.0.1");
    ASSERT_EQ(address14.host(), "127.0.0.1");
    ASSERT_EQ(address14.family(), AF_INET);
    ASSERT_EQ(address14.port(), 80);

    Address address15("www.example.com");
    ASSERT_EQ(address15.host(), "93.184.216.34");
    ASSERT_EQ(address15.family(), AF_INET);
    ASSERT_EQ(address15.port(), 80);

    Address address16(IP(127, 0, 0, 1), Port(8080));
    ASSERT_EQ(address16.host(), "127.0.0.1");
    ASSERT_EQ(address16.family(), AF_INET);
    ASSERT_EQ(address16.port(), 8080);

    Address address17(IP::any(), Port(8080));
    ASSERT_EQ(address17.host(), "0.0.0.0");
    ASSERT_EQ(address17.family(), AF_INET);
    ASSERT_EQ(address17.port(), 8080);

    Address address18(IP(2, 0, 0, 0, 0, 0, 0, 1), Port(8080));
    ASSERT_EQ(address18.host(), "2::1");
    ASSERT_EQ(address18.family(), AF_INET6);
    ASSERT_EQ(address18.port(), 8080);

    Address address19(IP::any(true), Port(8080));
    ASSERT_EQ(address19.host(), "::");
    ASSERT_EQ(address19.family(), AF_INET6);
    ASSERT_EQ(address19.port(), 8080);

    Address address20(IP::loopback(true), Port(8080));
    ASSERT_EQ(address20.host(), "::1");
    ASSERT_EQ(address20.family(), AF_INET6);
    ASSERT_EQ(address20.port(), 8080);

    Address address21(IP::loopback(), Port(8080));
    ASSERT_EQ(address21.host(), "127.0.0.1");
    ASSERT_EQ(address21.family(), AF_INET);
    ASSERT_EQ(address21.port(), 8080);

    Address address22("[2001:0DB8:AABB:CCDD:EEFF:0011:2233:4455]");
    ASSERT_EQ(address22.host(), "2001:db8:aabb:ccdd:eeff:11:2233:4455");
    ASSERT_EQ(address22.family(), AF_INET6);
    ASSERT_EQ(address22.port(), 80);

    Address address23("[::]");
    ASSERT_EQ(address23.host(), "::");
    ASSERT_EQ(address23.family(), AF_INET6);
    ASSERT_EQ(address23.port(), 80);
}

TEST(net_test, invalid_address)
{
    ASSERT_THROW(Address("127.0.0.1:9999999"), std::invalid_argument);
    ASSERT_THROW(Address("127.0.0.1:"), std::invalid_argument);
    ASSERT_THROW(Address("127.0.0.1:-10"), std::invalid_argument);

    ASSERT_THROW(Address("[GGGG:GGGG:GGGG:GGGG:GGGG:GGGG:GGGG:GGGG]:8080");
                 , std::invalid_argument);
    ASSERT_THROW(Address("[::GGGG]:8080");, std::invalid_argument);
    ASSERT_THROW(Address("256.256.256.256:8080");, std::invalid_argument);
    ASSERT_THROW(Address("1.0.0.256:8080");, std::invalid_argument);
}

TEST(net_test, address_parser)
{
    AddressParser ap1("127.0.0.1:80");
    ASSERT_EQ(ap1.rawHost(), "127.0.0.1");
    ASSERT_EQ(ap1.rawPort(), "80");
    ASSERT_EQ(ap1.family(), AF_INET);
    ASSERT_EQ(ap1.hasColon(), true);

    AddressParser ap2("example.com");
    ASSERT_EQ(ap2.rawHost(), "example.com");
    ASSERT_EQ(ap2.rawPort(), "");
    ASSERT_EQ(ap2.family(), AF_INET);
    ASSERT_EQ(ap2.hasColon(), false);

    AddressParser ap3("[2001:0DB8:AABB:CCDD:EEFF:0011:2233:4455]:8080");
    ASSERT_EQ(ap3.rawHost(), "[2001:0DB8:AABB:CCDD:EEFF:0011:2233:4455]");
    ASSERT_EQ(ap3.rawPort(), "8080");
    ASSERT_EQ(ap3.family(), AF_INET6);

    ASSERT_THROW(AddressParser("127.0.0.1:");, std::invalid_argument);
    ASSERT_THROW(AddressParser("[::]:");, std::invalid_argument);
}
