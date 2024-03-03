/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* tcp.cc
   Mathieu Stefani, 05 novembre 2015

   TCP
*/

#include <pistache/peer.h>
#include <pistache/tcp.h>

namespace Pistache::Tcp
{

    Handler::Handler()
        : transport_(nullptr)
    { }

    Handler::~Handler() = default;

    void Handler::associateTransport(Transport* transport)
    {
        transport_ = transport;
    }

    void Handler::onConnection(const std::shared_ptr<Tcp::Peer>& /*peer*/)
    { }

    void Handler::onDisconnection(const std::shared_ptr<Tcp::Peer>& /*peer*/)
    { }

} // namespace Pistache::Tcp
