/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 15 juin 2016

   Implementation of the Reactor
*/

#include <pistache/reactor.h>

#include <array>
#include <atomic>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

using namespace std::string_literals;

namespace Pistache::Aio
{

    class Reactor::Impl
    {
    public:
        Impl(Reactor* reactor)
            : reactor_(reactor)
        { }

        virtual ~Impl() = default;

        virtual Reactor::Key addHandler(const std::shared_ptr<Handler>& handler,
                                        bool setKey)
            = 0;

        virtual std::vector<std::shared_ptr<Handler>>
        handlers(const Reactor::Key& key) const = 0;

        virtual void registerFd(const Reactor::Key& key, Fd fd,
                                Polling::NotifyOn interest, Polling::Tag tag,
                                Polling::Mode mode = Polling::Mode::Level)
            = 0;

        virtual void registerFdOneShot(const Reactor::Key& key, Fd fd,
                                       Polling::NotifyOn interest, Polling::Tag tag,
                                       Polling::Mode mode = Polling::Mode::Level)
            = 0;

        virtual void modifyFd(const Reactor::Key& key, Fd fd,
                              Polling::NotifyOn interest, Polling::Tag tag,
                              Polling::Mode mode = Polling::Mode::Level)
            = 0;

        virtual void removeFd(const Reactor::Key& key, Fd fd) = 0;

        virtual void runOnce() = 0;
        virtual void run()     = 0;

        virtual void shutdown() = 0;

        Reactor* reactor_;
    };

    /* Synchronous implementation of the reactor that polls in the context
 * of the same thread
 */
    class SyncImpl : public Reactor::Impl
    {
    public:
        explicit SyncImpl(Reactor* reactor)
            : Reactor::Impl(reactor)
            , handlers_()
            , shutdown_()
            , shutdownFd()
            , poller()
        {
            shutdownFd.bind(poller);
        }

        Reactor::Key addHandler(const std::shared_ptr<Handler>& handler,
                                bool setKey = true) override
        {
            handler->registerPoller(poller);

            handler->reactor_ = reactor_;

            auto key = handlers_.add(handler);
            if (setKey)
                handler->key_ = key;

            return key;
        }

        std::shared_ptr<Handler> handler(const Reactor::Key& key) const
        {
            return handlers_.at(key.data());
        }

        std::vector<std::shared_ptr<Handler>>
        handlers(const Reactor::Key& key) const override
        {
            std::vector<std::shared_ptr<Handler>> res;

            res.push_back(handler(key));
            return res;
        }

        void registerFd(const Reactor::Key& key, Fd fd, Polling::NotifyOn interest,
                        Polling::Tag tag,
                        Polling::Mode mode = Polling::Mode::Level) override
        {

            auto pollTag = encodeTag(key, tag);
            poller.addFd(fd, Flags<Polling::NotifyOn>(interest), pollTag, mode);
        }

        void registerFdOneShot(const Reactor::Key& key, Fd fd,
                               Polling::NotifyOn interest, Polling::Tag tag,
                               Polling::Mode mode = Polling::Mode::Level) override
        {

            auto pollTag = encodeTag(key, tag);
            poller.addFdOneShot(fd, Flags<Polling::NotifyOn>(interest), pollTag, mode);
        }

        void modifyFd(const Reactor::Key& key, Fd fd, Polling::NotifyOn interest,
                      Polling::Tag tag,
                      Polling::Mode mode = Polling::Mode::Level) override
        {

            auto pollTag = encodeTag(key, tag);
            poller.rearmFd(fd, Flags<Polling::NotifyOn>(interest), pollTag, mode);
        }

        void removeFd(const Reactor::Key& /*key*/, Fd fd) override
        {
            poller.removeFd(fd);
        }

        void runOnce() override
        {
            if (handlers_.empty())
                throw std::runtime_error("You need to set at least one handler");

            for (;;)
            {
                std::vector<Polling::Event> events;
                int ready_fds = poller.poll(events);

                switch (ready_fds)
                {
                case -1:
                    break;
                case 0:
                    break;
                default:
                    if (shutdown_)
                        return;

                    handleFds(std::move(events));
                }
            }
        }

        void run() override
        {
            handlers_.forEachHandler([](const std::shared_ptr<Handler> handler) {
                handler->context_.tid = std::this_thread::get_id();
            });

            while (!shutdown_)
                runOnce();
        }

        void shutdown() override
        {
            shutdown_.store(true);
            shutdownFd.notify();
        }

        static constexpr size_t MaxHandlers() { return HandlerList::MaxHandlers; }

    private:
        static Polling::Tag encodeTag(const Reactor::Key& key, Polling::Tag tag)
        {
            uint64_t value = tag.value();
            return HandlerList::encodeTag(key, value);
        }

        static std::pair<size_t, uint64_t> decodeTag(const Polling::Tag& tag)
        {
            return HandlerList::decodeTag(tag);
        }

        void handleFds(std::vector<Polling::Event> events) const
        {
            // Fast-path: if we only have one handler, do not bother scanning the fds to
            // find the right handlers
            if (handlers_.size() == 1)
                handlers_.at(0)->onReady(FdSet(std::move(events)));
            else
            {
                std::unordered_map<std::shared_ptr<Handler>, std::vector<Polling::Event>>
                    fdHandlers;

                for (auto& event : events)
                {
                    size_t index;
                    uint64_t value;

                    std::tie(index, value) = decodeTag(event.tag);
                    auto handler_          = handlers_.at(index);
                    auto& evs              = fdHandlers.at(handler_);
                    evs.push_back(std::move(event));
                }

                for (auto& data : fdHandlers)
                {
                    data.first->onReady(FdSet(std::move(data.second)));
                }
            }
        }

        struct HandlerList
        {

            // We are using the highest 8 bits of the fd to encode the index of the
            // handler, which gives us a maximum of 2**8 - 1 handler, 255
            static constexpr size_t HandlerBits  = 8;
            static constexpr size_t HandlerShift = sizeof(uint64_t) - HandlerBits;
            static constexpr uint64_t DataMask   = uint64_t(-1) >> HandlerBits;

            static constexpr size_t MaxHandlers = (1 << HandlerBits) - 1;

            HandlerList()
                : handlers()
                , index_()
            {
                std::fill(std::begin(handlers), std::end(handlers), nullptr);
            }

            HandlerList(const HandlerList& other) = delete;
            HandlerList& operator=(const HandlerList& other) = delete;

            HandlerList(HandlerList&& other) = default;
            HandlerList& operator=(HandlerList&& other) = default;

            HandlerList clone() const
            {
                HandlerList list;

                for (size_t i = 0; i < index_; ++i)
                {
                    list.handlers.at(i) = handlers.at(i)->clone();
                }
                list.index_ = index_;

                return list;
            }

            Reactor::Key add(const std::shared_ptr<Handler>& handler)
            {
                if (index_ == MaxHandlers)
                    throw std::runtime_error("Maximum handlers reached");

                Reactor::Key key(index_);
                handlers.at(index_++) = handler;

                return key;
            }

            std::shared_ptr<Handler> operator[](size_t index) const
            {
                return handlers.at(index);
            }

            std::shared_ptr<Handler> at(size_t index) const
            {
                if (index >= index_)
                    throw std::runtime_error("Attempting to retrieve invalid handler");

                return handlers.at(index);
            }

            bool empty() const { return index_ == 0; }

            size_t size() const { return index_; }

            static Polling::Tag encodeTag(const Reactor::Key& key, uint64_t value)
            {
                auto index = key.data();
                // The reason why we are using the most significant bits to encode
                // the index of the handler is that in the fast path, we won't need
                // to shift the value to retrieve the fd if there is only one handler as
                // all the bits will already be set to 0.
                auto encodedValue = (index << HandlerShift) | value;
                return Polling::Tag(encodedValue);
            }

            static std::pair<size_t, uint64_t> decodeTag(const Polling::Tag& tag)
            {
                auto value   = tag.value();
                size_t index = value >> HandlerShift;
                uint64_t fd  = value & DataMask;

                return std::make_pair(index, fd);
            }

            template <typename Func>
            void forEachHandler(Func func) const
            {
                for (size_t i = 0; i < index_; ++i)
                    func(handlers.at(i));
            }

        private:
            std::array<std::shared_ptr<Handler>, MaxHandlers> handlers;
            size_t index_;
        };

        HandlerList handlers_;

        std::atomic<bool> shutdown_;
        NotifyFd shutdownFd;

        Polling::Epoll poller;
    };

    /* Asynchronous implementation of the reactor that spawns a number N of threads
 * and creates a polling fd per thread
 *
 * Implementation detail:
 *
 *  Here is how it works: the implementation simply starts a synchronous variant
 *  of the implementation in its own std::thread. When adding an handler, it
 * will add a clone() of the handler to every worker (thread), and assign its
 * own key to the handler. Here is where things start to get interesting. Here
 * is how the key encoding works for every handler:
 *
 *  [     handler idx      ] [       worker idx         ]
 *  ------------------------ ----------------------------
 *       ^ 32 bits                   ^ 32 bits
 *  -----------------------------------------------------
 *                       ^ 64 bits
 *
 * Since we have up to 64 bits of data for every key, we encode the index of the
 * handler that has been assigned by the SyncImpl in the upper 32 bits, and
 * encode the index of the worker thread in the lowest 32 bits.
 *
 * When registering a fd for a given key, the AsyncImpl then knows which worker
 * to use by looking at the lowest 32 bits of the Key's data. The SyncImpl will
 * then use the highest 32 bits to retrieve the index of the handler.
 */

    class AsyncImpl : public Reactor::Impl
    {
    public:
        static constexpr uint32_t KeyMarker = 0xBADB0B;

        AsyncImpl(Reactor* reactor, size_t threads, const std::string& threadsName)
            : Reactor::Impl(reactor)
        {

            if (threads > SyncImpl::MaxHandlers())
                throw std::runtime_error("Too many worker threads requested (max "s + std::to_string(SyncImpl::MaxHandlers()) + ")."s);

            for (size_t i = 0; i < threads; ++i)
                workers_.emplace_back(std::make_unique<Worker>(reactor, threadsName));
        }

        Reactor::Key addHandler(const std::shared_ptr<Handler>& handler,
                                bool) override
        {

            std::array<Reactor::Key, SyncImpl::MaxHandlers()> keys;

            for (size_t i = 0; i < workers_.size(); ++i)
            {
                auto& wrk = workers_.at(i);

                auto cl     = handler->clone();
                auto key    = wrk->sync->addHandler(cl, false /* setKey */);
                auto newKey = encodeKey(key, static_cast<uint32_t>(i));
                cl->key_    = newKey;

                keys.at(i) = key;
            }

            auto data = keys.at(0).data() << 32 | KeyMarker;

            return Reactor::Key(data);
        }

        std::vector<std::shared_ptr<Handler>>
        handlers(const Reactor::Key& key) const override
        {

            const std::pair<uint32_t, uint32_t> idx_marker = decodeKey(key);
            if (idx_marker.second != KeyMarker)
                throw std::runtime_error("Invalid key");

            Reactor::Key originalKey(idx_marker.first);

            std::vector<std::shared_ptr<Handler>> res;
            res.reserve(workers_.size());
            for (const auto& wrk : workers_)
            {
                res.push_back(wrk->sync->handler(originalKey));
            }

            return res;
        }

        void registerFd(const Reactor::Key& key, Fd fd, Polling::NotifyOn interest,
                        Polling::Tag tag,
                        Polling::Mode mode = Polling::Mode::Level) override
        {
            dispatchCall(key, &SyncImpl::registerFd, fd, interest, tag, mode);
        }

        void registerFdOneShot(const Reactor::Key& key, Fd fd,
                               Polling::NotifyOn interest, Polling::Tag tag,
                               Polling::Mode mode = Polling::Mode::Level) override
        {
            dispatchCall(key, &SyncImpl::registerFdOneShot, fd, interest, tag, mode);
        }

        void modifyFd(const Reactor::Key& key, Fd fd, Polling::NotifyOn interest,
                      Polling::Tag tag,
                      Polling::Mode mode = Polling::Mode::Level) override
        {
            dispatchCall(key, &SyncImpl::modifyFd, fd, interest, tag, mode);
        }

        void removeFd(const Reactor::Key& key, Fd fd) override
        {
            dispatchCall(key, &SyncImpl::removeFd, fd);
        }

        void runOnce() override { }

        void run() override
        {
            for (auto& wrk : workers_)
                wrk->run();
        }

        void shutdown() override
        {
            for (auto& wrk : workers_)
                wrk->shutdown();
        }

    private:
        static Reactor::Key encodeKey(const Reactor::Key& originalKey,
                                      uint32_t value)
        {
            auto data     = originalKey.data();
            auto newValue = data << 32 | value;
            return Reactor::Key(newValue);
        }

        static std::pair<uint32_t, uint32_t>
        decodeKey(const Reactor::Key& encodedKey)
        {
            auto data = encodedKey.data();
            auto hi   = static_cast<uint32_t>(data >> 32);
            auto lo   = static_cast<uint32_t>(data & 0xFFFFFFFF);
            return std::make_pair(hi, lo);
        }

#define CALL_MEMBER_FN(obj, pmf) (obj->*(pmf))

        template <typename Func, typename... Args>
        void dispatchCall(const Reactor::Key& key, Func func, Args&&... args) const
        {
            auto decoded    = decodeKey(key);
            const auto& wrk = workers_.at(decoded.second);

            Reactor::Key originalKey(decoded.first);
            CALL_MEMBER_FN(wrk->sync.get(), func)
            (originalKey, std::forward<Args>(args)...);
        }

#undef CALL_MEMBER_FN

        struct Worker
        {

            explicit Worker(Reactor* reactor, const std::string& threadsName)
                : thread()
                , sync(new SyncImpl(reactor))
                , threadsName_(threadsName)
            { }

            ~Worker()
            {
                if (thread.joinable())
                    thread.join();
            }

            void run()
            {
                thread = std::thread([=]() {
                    if (!threadsName_.empty())
                    {
                        pthread_setname_np(pthread_self(),
                                           threadsName_.substr(0, 15).c_str());
                    }
                    sync->run();
                });
            }

            void shutdown() { sync->shutdown(); }

            std::thread thread;
            std::unique_ptr<SyncImpl> sync;
            std::string threadsName_;
        };

        std::vector<std::unique_ptr<Worker>> workers_;
    };

    Reactor::Key::Key()
        : data_(0)
    { }

    Reactor::Key::Key(uint64_t data)
        : data_(data)
    { }

    Reactor::Reactor() = default;

    Reactor::~Reactor() = default;

    std::shared_ptr<Reactor> Reactor::create()
    {
        return std::make_shared<Reactor>();
    }

    void Reactor::init()
    {
        SyncContext context;
        init(context);
    }

    void Reactor::init(const ExecutionContext& context)
    {
        impl_.reset(context.makeImpl(this));
    }

    Reactor::Key Reactor::addHandler(const std::shared_ptr<Handler>& handler)
    {
        return impl()->addHandler(handler, true);
    }

    std::vector<std::shared_ptr<Handler>>
    Reactor::handlers(const Reactor::Key& key)
    {
        return impl()->handlers(key);
    }

    void Reactor::registerFd(const Reactor::Key& key, Fd fd,
                             Polling::NotifyOn interest, Polling::Tag tag,
                             Polling::Mode mode)
    {
        impl()->registerFd(key, fd, interest, tag, mode);
    }

    void Reactor::registerFdOneShot(const Reactor::Key& key, Fd fd,
                                    Polling::NotifyOn interest, Polling::Tag tag,
                                    Polling::Mode mode)
    {
        impl()->registerFdOneShot(key, fd, interest, tag, mode);
    }

    void Reactor::registerFd(const Reactor::Key& key, Fd fd,
                             Polling::NotifyOn interest, Polling::Mode mode)
    {
        impl()->registerFd(key, fd, interest, Polling::Tag(fd), mode);
    }

    void Reactor::registerFdOneShot(const Reactor::Key& key, Fd fd,
                                    Polling::NotifyOn interest,
                                    Polling::Mode mode)
    {
        impl()->registerFdOneShot(key, fd, interest, Polling::Tag(fd), mode);
    }

    void Reactor::modifyFd(const Reactor::Key& key, Fd fd,
                           Polling::NotifyOn interest, Polling::Tag tag,
                           Polling::Mode mode)
    {
        impl()->modifyFd(key, fd, interest, tag, mode);
    }

    void Reactor::modifyFd(const Reactor::Key& key, Fd fd,
                           Polling::NotifyOn interest, Polling::Mode mode)
    {
        impl()->modifyFd(key, fd, interest, Polling::Tag(fd), mode);
    }

    void Reactor::removeFd(const Reactor::Key& key, Fd fd)
    {
        impl()->removeFd(key, fd);
    }

    void Reactor::run() { impl()->run(); }

    void Reactor::shutdown()
    {
        if (impl_)
            impl()->shutdown();
    }

    void Reactor::runOnce() { impl()->runOnce(); }

    Reactor::Impl* Reactor::impl() const
    {
        if (!impl_)
            throw std::runtime_error(
                "Invalid object state, you should call init() before.");

        return impl_.get();
    }

    Reactor::Impl* SyncContext::makeImpl(Reactor* reactor) const
    {
        return new SyncImpl(reactor);
    }

    Reactor::Impl* AsyncContext::makeImpl(Reactor* reactor) const
    {
        return new AsyncImpl(reactor, threads_, threadsName_);
    }

    AsyncContext AsyncContext::singleThreaded() { return AsyncContext(1); }

} // namespace Pistache::Aio
