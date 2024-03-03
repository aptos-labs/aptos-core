/*
 * SPDX-FileCopyrightText: 2017 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* traqnsport.cc
   Mathieu Stefani, 02 July 2017

   TCP transport handling

*/

#include <sys/sendfile.h>
#include <sys/timerfd.h>

#include <pistache/os.h>
#include <pistache/peer.h>
#include <pistache/tcp.h>
#include <pistache/transport.h>
#include <pistache/utils.h>

namespace Pistache::Tcp
{
    using namespace Polling;

    Transport::Transport(const std::shared_ptr<Tcp::Handler>& handler)
    {
        init(handler);
    }

    void Transport::init(const std::shared_ptr<Tcp::Handler>& handler)
    {
        handler_ = handler;
        handler_->associateTransport(this);
    }

    std::shared_ptr<Aio::Handler> Transport::clone() const
    {
        return std::make_shared<Transport>(handler_->clone());
    }

    void Transport::flush()
    {
        handleWriteQueue(true);
    }

    void Transport::registerPoller(Polling::Epoll& poller)
    {
        writesQueue.bind(poller);
        timersQueue.bind(poller);
        peersQueue.bind(poller);
        notifier.bind(poller);
    }

    void Transport::handleNewPeer(const std::shared_ptr<Tcp::Peer>& peer)
    {
        auto ctx                   = context();
        const bool isInRightThread = std::this_thread::get_id() == ctx.thread();
        if (!isInRightThread)
        {
            PeerEntry entry(peer);
            peersQueue.push(std::move(entry));
        }
        else
        {
            handlePeer(peer);
        }
        int fd = peer->fd();
        {
            Guard guard(toWriteLock);
            toWrite.emplace(fd, std::deque<WriteEntry> {});
        }
    }

    void Transport::onReady(const Aio::FdSet& fds)
    {
        for (const auto& entry : fds)
        {
            if (entry.getTag() == writesQueue.tag())
            {
                handleWriteQueue();
            }
            else if (entry.getTag() == timersQueue.tag())
            {
                handleTimerQueue();
            }
            else if (entry.getTag() == peersQueue.tag())
            {
                handlePeerQueue();
            }
            else if (entry.getTag() == notifier.tag())
            {
                handleNotify();
            }

            else if (entry.isReadable())
            {
                auto tag = entry.getTag();
                if (isPeerFd(tag))
                {
                    auto& peer = getPeer(tag);
                    handleIncoming(peer);
                }
                else if (isTimerFd(tag))
                {
                    auto it      = timers.find(static_cast<decltype(timers)::key_type>(tag.value()));
                    auto& entry_ = it->second;
                    handleTimer(std::move(entry_));
                    timers.erase(it->first);
                }
            }
            else if (entry.isWritable())
            {
                auto tag = entry.getTag();
                auto fd  = static_cast<Fd>(tag.value());

                {
                    Guard guard(toWriteLock);
                    auto it = toWrite.find(fd);
                    if (it == std::end(toWrite))
                    {
                        throw std::runtime_error(
                            "Assertion Error: could not find write data");
                    }
                }

                reactor()->modifyFd(key(), fd, NotifyOn::Read, Polling::Mode::Edge);

                // Try to drain the queue
                asyncWriteImpl(fd);
            }
        }
    }

    void Transport::disarmTimer(Fd fd)
    {
        auto it = timers.find(fd);
        if (it == std::end(timers))
            throw std::runtime_error("Timer has not been armed");

        auto& entry = it->second;
        entry.disable();
    }

    void Transport::handleIncoming(const std::shared_ptr<Peer>& peer)
    {
        char buffer[Const::MaxBuffer] = { 0 };

        ssize_t totalBytes = 0;
        int fd             = peer->fd();

        for (;;)
        {

            ssize_t bytes;

#ifdef PISTACHE_USE_SSL
            if (peer->ssl() != NULL)
            {
                bytes = SSL_read((SSL*)peer->ssl(), buffer + totalBytes,
                                 static_cast<int>(Const::MaxBuffer - totalBytes));
            }
            else
            {
#endif /* PISTACHE_USE_SSL */
                bytes = recv(fd, buffer + totalBytes, Const::MaxBuffer - totalBytes, 0);
#ifdef PISTACHE_USE_SSL
            }
#endif /* PISTACHE_USE_SSL */

            if (bytes == -1)
            {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                {
                    if (totalBytes > 0)
                    {
                        handler_->onInput(buffer, totalBytes, peer);
                    }
                }
                else
                {
                    handlePeerDisconnection(peer);
                }
                break;
            }
            else if (bytes == 0)
            {
                handlePeerDisconnection(peer);
                break;
            }

            else
            {
                handler_->onInput(buffer, bytes, peer);
            }
        }
    }

    void Transport::handlePeerDisconnection(const std::shared_ptr<Peer>& peer)
    {
        handler_->onDisconnection(peer);

        removePeer(peer);
    }

    void Transport::removePeer(const std::shared_ptr<Peer>& peer)
    {
        int fd  = peer->fd();
        auto it = peers.find(fd);
        if (it == std::end(peers))
            throw std::runtime_error("Could not find peer to erase");

        peers.erase(it->first);

        {
            // Clean up buffers
            Guard guard(toWriteLock);
            toWrite.erase(fd);
        }

        // Don't rely on close deleting this FD from the epoll "interest" list.
        // This is needed in case the FD has been shared with another process.
        // Sharing should no longer happen by accident as SOCK_CLOEXEC is now set on
        // listener accept. This should then guarantee that the next call to
        // epoll_wait will not give us any events relating to this FD even if they
        // have been queued in the kernel since the last call to epoll_wait.
        reactor()->removeFd(key(), fd);

        close(fd);
    }

    void Transport::asyncWriteImpl(Fd fd)
    {
        bool stop = false;
        while (!stop)
        {
            std::unique_lock<std::mutex> lock(toWriteLock);

            auto it = toWrite.find(fd);

            // cleanup will have been handled by handlePeerDisconnection
            if (it == std::end(toWrite))
            {
                return;
            }
            auto& wq = it->second;
            if (wq.empty())
            {
                break;
            }

            auto& entry                       = wq.front();
            int flags                         = entry.flags;
            BufferHolder& buffer              = entry.buffer;
            Async::Deferred<ssize_t> deferred = std::move(entry.deferred);

            auto cleanUp = [&]() {
                wq.pop_front();
                if (wq.empty())
                {
                    toWrite.erase(fd);
                    reactor()->modifyFd(key(), fd, NotifyOn::Read, Polling::Mode::Edge);
                    stop = true;
                }
                lock.unlock();
            };

            size_t totalWritten = buffer.offset();
            for (;;)
            {
                ssize_t bytesWritten = 0;
                auto len             = buffer.size() - totalWritten;

                if (buffer.isRaw())
                {
                    auto raw        = buffer.raw();
                    const auto* ptr = raw.data().c_str() + totalWritten;
                    bytesWritten    = sendRawBuffer(fd, ptr, len, flags);
                }
                else
                {
                    auto file    = buffer.fd();
                    off_t offset = totalWritten;
                    bytesWritten = sendFile(fd, file, offset, len);
                }
                if (bytesWritten < 0)
                {
                    if (errno == EAGAIN || errno == EWOULDBLOCK)
                    {

                        auto bufferHolder = buffer.detach(totalWritten);

                        // pop_front kills buffer - so we cannot continue loop or use buffer
                        // after this point
                        wq.pop_front();
                        wq.push_front(WriteEntry(std::move(deferred), bufferHolder, flags));
                        reactor()->modifyFd(key(), fd, NotifyOn::Read | NotifyOn::Write,
                                            Polling::Mode::Edge);
                    }
                    // EBADF can happen when the HTTP parser, in the case of
                    // an error, closes fd before the entire request is processed.
                    // https://github.com/pistacheio/pistache/issues/501
                    else if (errno == EBADF || errno == EPIPE || errno == ECONNRESET)
                    {
                        wq.pop_front();
                        toWrite.erase(fd);
                        stop = true;
                    }
                    else
                    {
                        cleanUp();
                        deferred.reject(Pistache::Error::system("Could not write data"));
                    }
                    break;
                }
                else
                {
                    totalWritten += bytesWritten;
                    if (totalWritten >= buffer.size())
                    {
                        if (buffer.isFile())
                        {
                            // done with the file buffer, nothing else knows whether to
                            // close it with the way the code is written.
                            ::close(buffer.fd());
                        }

                        cleanUp();

                        // Cast to match the type of defered template
                        // to avoid a BadType exception
                        deferred.resolve(static_cast<ssize_t>(totalWritten));
                        break;
                    }
                }
            }
        }
    }

    ssize_t Transport::sendRawBuffer(Fd fd, const char* buffer, size_t len, int flags)
    {
        ssize_t bytesWritten = 0;

#ifdef PISTACHE_USE_SSL
        auto it_ = peers.find(fd);

        if (it_ == std::end(peers))
            throw std::runtime_error("No peer found for fd: " + std::to_string(fd));

        if (it_->second->ssl() != NULL)
        {
            auto ssl_    = static_cast<SSL*>(it_->second->ssl());
            bytesWritten = SSL_write(ssl_, buffer, static_cast<int>(len));
        }
        else
        {
#endif /* PISTACHE_USE_SSL */
            // MSG_NOSIGNAL is used to prevent SIGPIPE on client connection termination
            bytesWritten = ::send(fd, buffer, len, flags | MSG_NOSIGNAL);
#ifdef PISTACHE_USE_SSL
        }
#endif /* PISTACHE_USE_SSL */

        return bytesWritten;
    }

    ssize_t Transport::sendFile(Fd fd, Fd file, off_t offset, size_t len)
    {
        ssize_t bytesWritten = 0;

#ifdef PISTACHE_USE_SSL
        auto it_ = peers.find(fd);

        if (it_ == std::end(peers))
            throw std::runtime_error("No peer found for fd: " + std::to_string(fd));

        if (it_->second->ssl() != NULL)
        {
            auto ssl_    = static_cast<SSL*>(it_->second->ssl());
            bytesWritten = SSL_sendfile(ssl_, file, &offset, len);
        }
        else
        {
#endif /* PISTACHE_USE_SSL */
            bytesWritten = ::sendfile(fd, file, &offset, len);
#ifdef PISTACHE_USE_SSL
        }
#endif /* PISTACHE_USE_SSL */

        return bytesWritten;
    }

    void Transport::armTimerMs(Fd fd, std::chrono::milliseconds value,
                               Async::Deferred<uint64_t> deferred)
    {

        auto ctx                   = context();
        const bool isInRightThread = std::this_thread::get_id() == ctx.thread();
        TimerEntry entry(fd, value, std::move(deferred));

        if (!isInRightThread)
        {
            timersQueue.push(std::move(entry));
        }
        else
        {
            armTimerMsImpl(std::move(entry));
        }
    }

    void Transport::armTimerMsImpl(TimerEntry entry)
    {

        auto it = timers.find(entry.fd);
        if (it != std::end(timers))
        {
            entry.deferred.reject(std::runtime_error("Timer is already armed"));
            return;
        }

        itimerspec spec;
        spec.it_interval.tv_sec  = 0;
        spec.it_interval.tv_nsec = 0;

        if (entry.value.count() < 1000)
        {
            spec.it_value.tv_sec  = 0;
            spec.it_value.tv_nsec = std::chrono::duration_cast<std::chrono::nanoseconds>(entry.value)
                                        .count();
        }
        else
        {
            spec.it_value.tv_sec  = std::chrono::duration_cast<std::chrono::seconds>(entry.value).count();
            spec.it_value.tv_nsec = 0;
        }

        int res = timerfd_settime(entry.fd, 0, &spec, nullptr);
        if (res == -1)
        {
            entry.deferred.reject(Pistache::Error::system("Could not set timer time"));
            return;
        }

        reactor()->registerFdOneShot(key(), entry.fd, NotifyOn::Read,
                                     Polling::Mode::Edge);
        timers.insert(std::make_pair(entry.fd, std::move(entry)));
    }

    void Transport::handleWriteQueue(bool flush)
    {
        // Let's drain the queue
        for (;;)
        {
            auto write = writesQueue.popSafe();
            if (!write)
                break;

            auto fd = write->peerFd;
            if (!isPeerFd(fd))
                continue;

            {
                Guard guard(toWriteLock);
                toWrite[fd].push_back(std::move(*write));
            }

            reactor()->modifyFd(key(), fd, NotifyOn::Read | NotifyOn::Write,
                                Polling::Mode::Edge);

            if (flush)
                asyncWriteImpl(fd);
        }
    }

    void Transport::handleTimerQueue()
    {
        for (;;)
        {
            auto timer = timersQueue.popSafe();
            if (!timer)
                break;

            armTimerMsImpl(std::move(*timer));
        }
    }

    void Transport::handlePeerQueue()
    {
        for (;;)
        {
            auto data = peersQueue.popSafe();
            if (!data)
                break;

            handlePeer(data->peer);
        }
    }

    void Transport::handlePeer(const std::shared_ptr<Peer>& peer)
    {
        int fd = peer->fd();
        peers.insert(std::make_pair(fd, peer));

        peer->associateTransport(this);

        handler_->onConnection(peer);
        reactor()->registerFd(key(), fd, NotifyOn::Read | NotifyOn::Shutdown,
                              Polling::Mode::Edge);
    }

    void Transport::handleNotify()
    {
        while (this->notifier.tryRead())
            ;

        rusage now;

        auto res = getrusage(RUSAGE_THREAD, &now);
        if (res == -1)
            loadRequest_.reject(std::runtime_error("Could not compute usage"));

        loadRequest_.resolve(now);
        loadRequest_.clear();
    }

    void Transport::handleTimer(TimerEntry entry)
    {
        if (entry.isActive())
        {
            uint64_t numWakeups;
            auto res = ::read(entry.fd, &numWakeups, sizeof numWakeups);
            if (res == -1)
            {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                    return;
                else
                    entry.deferred.reject(
                        Pistache::Error::system("Could not read timerfd"));
            }
            else
            {
                if (res != sizeof(numWakeups))
                {
                    entry.deferred.reject(
                        Pistache::Error("Read invalid number of bytes for timer fd: " + std::to_string(entry.fd)));
                }
                else
                {
                    entry.deferred.resolve(numWakeups);
                }
            }
        }
    }

    bool Transport::isPeerFd(Fd fd) const
    {
        return peers.find(fd) != std::end(peers);
    }

    bool Transport::isTimerFd(Fd fd) const
    {
        return timers.find(fd) != std::end(timers);
    }

    bool Transport::isPeerFd(Polling::Tag tag) const
    {
        return isPeerFd(static_cast<Fd>(tag.value()));
    }
    bool Transport::isTimerFd(Polling::Tag tag) const
    {
        return isTimerFd(static_cast<Fd>(tag.value()));
    }

    std::shared_ptr<Peer>& Transport::getPeer(Fd fd)
    {
        auto it = peers.find(fd);
        if (it == std::end(peers))
        {
            throw std::runtime_error("No peer found for fd: " + std::to_string(fd));
        }
        return it->second;
    }

    std::shared_ptr<Peer>& Transport::getPeer(Polling::Tag tag)
    {
        return getPeer(static_cast<Fd>(tag.value()));
    }

    std::deque<std::shared_ptr<Peer>> Transport::getAllPeer()
    {
        std::deque<std::shared_ptr<Peer>> dqPeers;
        for (const auto& peerPair : peers)
        {
            if (isPeerFd(peerPair.first))
            {
                dqPeers.push_back(peerPair.second);
            }
        }
        return dqPeers;
    }

} // namespace Pistache::Tcp
