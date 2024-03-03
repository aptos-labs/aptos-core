/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* peer.cc
   Mathieu Stefani, 12 August 2015

*/

#include <iostream>
#include <stdexcept>

#include <arpa/inet.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/types.h>

#include <pistache/async.h>
#include <pistache/peer.h>
#include <pistache/transport.h>

namespace Pistache::Tcp
{
    namespace
    {
        struct ConcretePeer : Peer
        {
            ConcretePeer() = default;
            [[maybe_unused]] ConcretePeer(Fd fd, const Address& addr, void* ssl)
                : Peer(fd, addr, ssl)
            { }
        };
    } // namespace

    Peer::Peer(Fd fd, const Address& addr, void* ssl)
        : fd_(fd)
        , addr(addr)
        , ssl_(ssl)
        , id_(getUniqueId())
    { }

    Peer::~Peer()
    {
#ifdef PISTACHE_USE_SSL
        if (ssl_)
            SSL_free(static_cast<SSL*>(ssl_));
#endif /* PISTACHE_USE_SSL */
    }

    std::shared_ptr<Peer> Peer::Create(Fd fd, const Address& addr)
    {
        return std::make_shared<ConcretePeer>(fd, addr, nullptr);
    }

    std::shared_ptr<Peer> Peer::CreateSSL(Fd fd, const Address& addr, void* ssl)
    {
        return std::make_shared<ConcretePeer>(fd, addr, ssl);
    }

    const Address& Peer::address() const { return addr; }

    void Peer::setIdle(bool bIdle) { isIdle_ = bIdle; }
    bool Peer::isIdle() const { return isIdle_; }

    const std::string& Peer::hostname()
    {
        if (hostname_.empty())
        {
            char host[NI_MAXHOST];
            struct sockaddr_in sa;
            sa.sin_family = AF_INET;
            if (inet_pton(AF_INET, addr.host().c_str(), &sa.sin_addr) == 0)
            {
                hostname_ = addr.host();
            }
            else
            {
                if (!getnameinfo(reinterpret_cast<struct sockaddr*>(&sa), sizeof(sa), host, sizeof(host),
                                 nullptr, 0 // Service info
                                 ,
                                 NI_NAMEREQD // Raise an error if name resolution failed
                                 ))
                {
                    hostname_.assign(static_cast<char*>(host));
                }
            }
        }
        return hostname_;
    }

    void* Peer::ssl() const { return ssl_; }
    size_t Peer::getID() const { return id_; }

    int Peer::fd() const
    {
        if (fd_ == -1)
        {
            throw std::runtime_error("The peer has no associated fd");
        }

        return fd_;
    }

    void Peer::putData(std::string name, std::shared_ptr<void> data)
    {
        auto it = data_.find(name);
        if (it != std::end(data_))
        {
            throw std::runtime_error("The data already exists");
        }

        data_.insert(std::make_pair(std::move(name), std::move(data)));
    }

    std::shared_ptr<void> Peer::getData(std::string name) const
    {
        auto data = tryGetData(std::move(name));
        if (data == nullptr)
        {
            throw std::runtime_error("The data does not exist");
        }

        return data;
    }

    std::shared_ptr<void> Peer::tryGetData(std::string name) const
    {
        auto it = data_.find(name);
        if (it == std::end(data_))
            return nullptr;

        return it->second;
    }

    Async::Promise<ssize_t> Peer::send(const RawBuffer& buffer, int flags)
    {
        return transport()->asyncWrite(fd_, buffer, flags);
    }

    std::ostream& operator<<(std::ostream& os, Peer& peer)
    {
        const auto& addr = peer.address();
        os << "Peer " << &peer << " (id=" << peer.getID() << ", address=" << addr
           << ", hostname=" << peer.hostname() << ", fd=" << peer.fd() << ")";
        return os;
    }

    void Peer::associateTransport(Transport* transport) { transport_ = transport; }

    Transport* Peer::transport() const
    {
        if (!transport_)
            throw std::logic_error("Orphaned peer");

        return transport_;
    }

    size_t Peer::getUniqueId()
    {
        static std::atomic<size_t> idCounter { 0 };
        return idCounter++;
    }

} // namespace Pistache::Tcp
