/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 26 janvier 2016

   Transport TCP layer
*/

#pragma once

#include <pistache/async.h>
#include <pistache/mailbox.h>
#include <pistache/reactor.h>
#include <pistache/stream.h>

#include <chrono>
#include <deque>
#include <memory>
#include <mutex>
#include <optional>
#include <unordered_map>

namespace Pistache::Tcp
{

    class Peer;
    class Handler;

    class Transport : public Aio::Handler
    {
    public:
        explicit Transport(const std::shared_ptr<Tcp::Handler>& handler);

        Transport(const Transport&)            = delete;
        Transport& operator=(const Transport&) = delete;

        void init(const std::shared_ptr<Tcp::Handler>& handler);

        void registerPoller(Polling::Epoll& poller) override;

        void handleNewPeer(const std::shared_ptr<Peer>& peer);
        void onReady(const Aio::FdSet& fds) override;

        template <typename Buf>
        Async::Promise<ssize_t> asyncWrite(Fd fd, const Buf& buffer, int flags = 0)
        {
            // Always enqueue reponses for sending. Giving preference to consumer
            // context means chunked responses could be sent out of order.
            return Async::Promise<ssize_t>(
                [=](Async::Deferred<ssize_t> deferred) mutable {
                    BufferHolder holder { buffer };
                    WriteEntry write(std::move(deferred), std::move(holder), fd, flags);
                    writesQueue.push(std::move(write));
                });
        }

        Async::Promise<rusage> load()
        {
            return Async::Promise<rusage>([=](Async::Deferred<rusage> deferred) {
                loadRequest_ = std::move(deferred);
                notifier.notify();
            });
        }

        template <typename Duration>
        void armTimer(Fd fd, Duration timeout, Async::Deferred<uint64_t> deferred)
        {
            armTimerMs(fd,
                       std::chrono::duration_cast<std::chrono::milliseconds>(timeout),
                       std::move(deferred));
        }

        void disarmTimer(Fd fd);

        std::shared_ptr<Aio::Handler> clone() const override;

        void flush();

        std::deque<std::shared_ptr<Peer>> getAllPeer();

    private:
        enum WriteStatus { FirstTry,
                           Retry };

        struct BufferHolder
        {
            enum Type { Raw,
                        File };

            explicit BufferHolder(const RawBuffer& buffer, off_t offset = 0)
                : _raw(buffer)
                , size_(buffer.size())
                , offset_(offset)
                , type(Raw)
            { }

            explicit BufferHolder(const FileBuffer& buffer, off_t offset = 0)
                : _fd(buffer.fd())
                , size_(buffer.size())
                , offset_(offset)
                , type(File)
            { }

            bool isFile() const { return type == File; }
            bool isRaw() const { return type == Raw; }
            size_t size() const { return size_; }
            size_t offset() const { return offset_; }

            Fd fd() const
            {
                if (!isFile())
                    throw std::runtime_error("Tried to retrieve fd of a non-filebuffer");
                return _fd;
            }

            RawBuffer raw() const
            {
                if (!isRaw())
                    throw std::runtime_error("Tried to retrieve raw data of a non-buffer");
                return _raw;
            }

            BufferHolder detach(size_t offset = 0)
            {
                if (!isRaw())
                    return BufferHolder(_fd, size_, offset);

                auto detached = _raw.copy(offset);
                return BufferHolder(detached);
            }

        private:
            BufferHolder(Fd fd, size_t size, off_t offset = 0)
                : _fd(fd)
                , size_(size)
                , offset_(offset)
                , type(File)
            { }

            RawBuffer _raw;
            Fd _fd;

            size_t size_  = 0;
            off_t offset_ = 0;
            Type type;
        };

        struct WriteEntry
        {
            WriteEntry(Async::Deferred<ssize_t> deferred_, BufferHolder buffer_,
                       Fd peerFd_, int flags_ = 0)
                : deferred(std::move(deferred_))
                , buffer(std::move(buffer_))
                , flags(flags_)
                , peerFd(peerFd_)
            { }

            Async::Deferred<ssize_t> deferred;
            BufferHolder buffer;
            int flags = 0;
            Fd peerFd = -1;
        };

        struct TimerEntry
        {
            TimerEntry(Fd fd_, std::chrono::milliseconds value_,
                       Async::Deferred<uint64_t> deferred_)
                : fd(fd_)
                , value(value_)
                , deferred(std::move(deferred_))
                , active()
            {
                active.store(true, std::memory_order_relaxed);
            }

            TimerEntry(TimerEntry&& other)
                : fd(other.fd)
                , value(other.value)
                , deferred(std::move(other.deferred))
                , active(other.active.load())
            { }

            void disable() { active.store(false, std::memory_order_relaxed); }

            bool isActive() const { return active.load(std::memory_order_relaxed); }

            Fd fd;
            std::chrono::milliseconds value;
            Async::Deferred<uint64_t> deferred;
            std::atomic<bool> active;
        };

        struct PeerEntry
        {
            explicit PeerEntry(std::shared_ptr<Peer> peer_)
                : peer(std::move(peer_))
            { }

            std::shared_ptr<Peer> peer;
        };
        using Lock  = std::mutex;
        using Guard = std::lock_guard<Lock>;

        PollableQueue<WriteEntry> writesQueue;
        std::unordered_map<Fd, std::deque<WriteEntry>> toWrite;
        Lock toWriteLock;

        PollableQueue<TimerEntry> timersQueue;
        std::unordered_map<Fd, TimerEntry> timers;

        PollableQueue<PeerEntry> peersQueue;

        Async::Deferred<rusage> loadRequest_;
        NotifyFd notifier;

        std::shared_ptr<Tcp::Handler> handler_;

    protected:
        void removePeer(const std::shared_ptr<Peer>& peer);
        std::unordered_map<Fd, std::shared_ptr<Peer>> peers;

    private:
        bool isPeerFd(Fd fd) const;
        bool isTimerFd(Fd fd) const;
        bool isPeerFd(Polling::Tag tag) const;
        bool isTimerFd(Polling::Tag tag) const;

        std::shared_ptr<Peer>& getPeer(Fd fd);
        std::shared_ptr<Peer>& getPeer(Polling::Tag tag);

        void armTimerMs(Fd fd, std::chrono::milliseconds value,
                        Async::Deferred<uint64_t> deferred);

        void armTimerMsImpl(TimerEntry entry);

        // This will attempt to drain the write queue for the fd
        void asyncWriteImpl(Fd fd);
        ssize_t sendRawBuffer(Fd fd, const char* buffer, size_t len, int flags);
        ssize_t sendFile(Fd fd, Fd file, off_t offset, size_t len);

        void handlePeerDisconnection(const std::shared_ptr<Peer>& peer);
        void handleIncoming(const std::shared_ptr<Peer>& peer);
        void handleWriteQueue(bool flush = false);
        void handleTimerQueue();
        void handlePeerQueue();
        void handleNotify();
        void handleTimer(TimerEntry entry);
        void handlePeer(const std::shared_ptr<Peer>& peer);
    };

} // namespace Pistache::Tcp
