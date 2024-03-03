/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 15 juin 2016

   A lightweight implementation of the Reactor design-pattern.

   The main goal of this component is to provide an solid abstraction
   that can be used internally and by client code to dispatch I/O events
   to callbacks and handlers, in an efficient way.
*/

#pragma once

#include <pistache/flags.h>
#include <pistache/net.h>
#include <pistache/os.h>
#include <pistache/prototype.h>

#include <sys/resource.h>
#include <sys/time.h>

#include <memory>
#include <thread>
#include <vector>

namespace Pistache::Aio
{

    // A set of fds that are ready
    class FdSet
    {
    public:
        FdSet() = delete;

        explicit FdSet(std::vector<Polling::Event>&& events)
            : events_()
        {
            events_.reserve(events.size());
            events_.insert(events_.end(), std::make_move_iterator(events.begin()),
                           std::make_move_iterator(events.end()));
        }

        struct Entry : private Polling::Event
        {
            Entry(Polling::Event&& event)
                : Polling::Event(std::move(event))
            { }

            bool isReadable() const { return flags.hasFlag(Polling::NotifyOn::Read); }
            bool isWritable() const { return flags.hasFlag(Polling::NotifyOn::Write); }
            bool isHangup() const { return flags.hasFlag(Polling::NotifyOn::Hangup); }

            Polling::Tag getTag() const { return this->tag; }
        };

        using iterator       = std::vector<Entry>::iterator;
        using const_iterator = std::vector<Entry>::const_iterator;

        size_t size() const { return events_.size(); }

        const Entry& at(size_t index) const { return events_.at(index); }

        const Entry& operator[](size_t index) const { return events_.at(index); }

        iterator begin() { return events_.begin(); }

        iterator end() { return events_.end(); }

        const_iterator begin() const { return events_.begin(); }

        const_iterator end() const { return events_.end(); }

    private:
        std::vector<Entry> events_;
    };

    class Handler;
    class ExecutionContext;

    class Reactor : public std::enable_shared_from_this<Reactor>
    {
    public:
        class Impl;

        Reactor();
        ~Reactor();

        struct Key
        {

            Key();

            friend class Reactor;
            friend class Impl;
            friend class SyncImpl;
            friend class AsyncImpl;

            uint64_t data() const { return data_; }

        private:
            explicit Key(uint64_t data);
            uint64_t data_;
        };

        static std::shared_ptr<Reactor> create();

        void init();
        void init(const ExecutionContext& context);

        Key addHandler(const std::shared_ptr<Handler>& handler);

        std::vector<std::shared_ptr<Handler>> handlers(const Key& key);

        void registerFd(const Key& key, Fd fd, Polling::NotifyOn interest,
                        Polling::Tag tag, Polling::Mode mode = Polling::Mode::Level);
        void registerFdOneShot(const Key& key, Fd fd, Polling::NotifyOn interest,
                               Polling::Tag tag,
                               Polling::Mode mode = Polling::Mode::Level);

        void registerFd(const Key& key, Fd fd, Polling::NotifyOn interest,
                        Polling::Mode mode = Polling::Mode::Level);
        void registerFdOneShot(const Key& key, Fd fd, Polling::NotifyOn interest,
                               Polling::Mode mode = Polling::Mode::Level);

        void modifyFd(const Key& key, Fd fd, Polling::NotifyOn interest,
                      Polling::Mode mode = Polling::Mode::Level);

        void modifyFd(const Key& key, Fd fd, Polling::NotifyOn interest,
                      Polling::Tag tag, Polling::Mode mode = Polling::Mode::Level);

        void removeFd(const Key& key, Fd fd);

        void runOnce();
        void run();

        void shutdown();

    private:
        Impl* impl() const;
        std::unique_ptr<Impl> impl_;
    };

    class ExecutionContext
    {
    public:
        virtual ~ExecutionContext()                             = default;
        virtual Reactor::Impl* makeImpl(Reactor* reactor) const = 0;
    };

    class SyncContext : public ExecutionContext
    {
    public:
        ~SyncContext() override = default;
        Reactor::Impl* makeImpl(Reactor* reactor) const override;
    };

    class AsyncContext : public ExecutionContext
    {
    public:
        explicit AsyncContext(size_t threads, const std::string& threadsName = "")
            : threads_(threads)
            , threadsName_(threadsName)
        { }

        ~AsyncContext() override = default;

        Reactor::Impl* makeImpl(Reactor* reactor) const override;

        static AsyncContext singleThreaded();

    private:
        size_t threads_;
        std::string threadsName_;
    };

    class Handler : public Prototype<Handler>
    {
    public:
        friend class Reactor;
        friend class SyncImpl;
        friend class AsyncImpl;

        Handler()
            : reactor_(nullptr)
            , context_()
            , key_()
        { }

        struct Context
        {
            friend class SyncImpl;

            Context()
                : tid()
            { }

            std::thread::id thread() const { return tid; }

        private:
            std::thread::id tid;
        };

        virtual void onReady(const FdSet& fds)              = 0;
        virtual void registerPoller(Polling::Epoll& poller) = 0;

        Reactor* reactor() const { return reactor_; }

        Context context() const { return context_; }

        Reactor::Key key() const { return key_; };

        ~Handler() override = default;

    private:
        Reactor* reactor_;
        Context context_;
        Reactor::Key key_;
    };

} // namespace Pistache::Aio
