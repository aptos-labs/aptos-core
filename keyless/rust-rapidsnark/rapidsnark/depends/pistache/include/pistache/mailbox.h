/*
 * Copyright (c) 2014 Datacratic.  All rights reserved.
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <atomic>
#include <stdexcept>

#include <array>
#include <sys/eventfd.h>
#include <unistd.h>

#include <pistache/common.h>
#include <pistache/os.h>

namespace Pistache
{

    static constexpr size_t CachelineSize = 64;
    typedef char cacheline_pad_t[CachelineSize];

    template <typename T>
    class Mailbox
    {
    public:
        Mailbox() { data.store(nullptr); }

        virtual ~Mailbox() = default;

        const T* get() const
        {
            if (isEmpty())
            {
                throw std::runtime_error("Can not retrieve mail from empty mailbox");
            }

            return data.load();
        }

        virtual T* post(T* newData)
        {
            T* old = data.load();
            while (!data.compare_exchange_weak(old, newData))
            {
            }

            return old;
        }

        virtual T* clear() { return data.exchange(nullptr); }

        bool isEmpty() const { return data == nullptr; }

    private:
        std::atomic<T*> data;
    };

    template <typename T>
    class PollableMailbox : public Mailbox<T>
    {
    public:
        PollableMailbox()
            : event_fd(-1)
        { }

        ~PollableMailbox()
        {
            if (event_fd != -1)
                close(event_fd);
        }

        bool isBound() const { return event_fd != -1; }

        Polling::Tag bind(Polling::Epoll& poller)
        {
            using namespace Polling;

            if (isBound())
            {
                throw std::runtime_error("The mailbox has already been bound");
            }

            event_fd = TRY_RET(eventfd(0, EFD_NONBLOCK));
            Tag tag_(event_fd);
            poller.addFd(event_fd, Flags<Polling::NotifyOn>(NotifyOn::Read), tag_);

            return tag_;
        }

        T* post(T* newData)
        {
            auto* _ret = Mailbox<T>::post(newData);

            if (isBound())
            {
                uint64_t val = 1;
                TRY(write(event_fd, &val, sizeof val));
            }

            return _ret;
        }

        T* clear()
        {
            auto ret = Mailbox<T>::clear();

            if (isBound())
            {
                uint64_t val;
                for (;;)
                {
                    ssize_t bytes = read(event_fd, &val, sizeof val);
                    if (bytes == -1)
                    {
                        if (errno == EAGAIN || errno == EWOULDBLOCK)
                            break;
                        else
                        {
                            // TODO
                        }
                    }
                }
            }

            return ret;
        }

        Polling::Tag tag() const
        {
            if (!isBound())
                throw std::runtime_error("Can not retrieve tag of an unbound mailbox");

            return Polling::Tag(event_fd);
        }

        void unbind(Polling::Epoll& poller)
        {
            if (event_fd == -1)
            {
                throw std::runtime_error("The mailbox is not bound");
            }

            poller.removeFd(event_fd);
            close(event_fd), event_fd = -1;
        }

    private:
        int event_fd;
    };

    /*
 * An unbounded MPSC lock-free queue. Usefull for efficient cross-thread message
 passing.

 * push() and pop() are wait-free.
 * Might replace the Mailbox implementation below

 * Design comes from
 http://www.1024cores.net/home/lock-free-algorithms/queues/non-intrusive-mpsc-node-based-queue
*/
    template <typename T>
    class Queue
    {
    public:
        struct Entry
        {
            friend class Queue;

            Entry()
                : storage()
                , next(nullptr)
            { }

            template <class U>
            explicit Entry(U&& u)
                : storage()
                , next(nullptr)
            {
                new (&storage) T(std::forward<U>(u));
            }

            const T& data() const { return *reinterpret_cast<const T*>(&storage); }

            T& data() { return *reinterpret_cast<T*>(&storage); }

        private:
            typedef typename std::aligned_storage<sizeof(T), alignof(T)>::type Storage;
            Storage storage;
            std::atomic<Entry*> next;
        };

        Queue()
            : head()
            , tail(nullptr)
        {
            auto* sentinel = new Entry;
            sentinel->next = nullptr;
            head.store(sentinel, std::memory_order_relaxed);
            tail = sentinel;
        }

        virtual ~Queue()
        {
            while (!empty())
            {
                Entry* e = pop();
                e->data().~T();
                delete e;
            }
            delete tail;
        }

        template <typename U>
        void push(U&& u)
        {
            Entry* entry = new Entry(std::forward<U>(u));
            // @Note: we're using SC atomics here (exchange will issue a full fence),
            // but I don't think we should bother relaxing them for now
            auto* prev = head.exchange(entry);
            prev->next = entry;
        }

        virtual Entry* pop()
        {
            auto* res  = tail;
            auto* next = res->next.load(std::memory_order_acquire);
            if (next)
            {
                // Since it's Single-Consumer, the store does not need to be atomic
                tail = next;
                new (&res->storage) T(std::move(next->data()));
                return res;
            }
            return nullptr;
        }

        bool empty() { return head == tail; }

        std::unique_ptr<T> popSafe()
        {
            std::unique_ptr<T> object;

            std::unique_ptr<Entry> entry(pop());
            if (entry)
            {
                object.reset(new T(std::move(entry->data())));
                entry->data().~T();
            }

            return object;
        }

    private:
        std::atomic<Entry*> head;
        Entry* tail;
    };

    template <typename T>
    class PollableQueue : public Queue<T>
    {
    public:
        typedef typename Queue<T>::Entry Entry;

        PollableQueue()
            : event_fd(-1)
        { }

        ~PollableQueue() override
        {
            if (event_fd != -1)
                close(event_fd);
        }

        bool isBound() const { return event_fd != -1; }

        Polling::Tag bind(Polling::Epoll& poller)
        {
            using namespace Polling;

            if (isBound())
            {
                throw std::runtime_error("The queue has already been bound");
            }

            event_fd = TRY_RET(eventfd(0, EFD_NONBLOCK));
            Tag tag_(event_fd);
            poller.addFd(event_fd, Flags<Polling::NotifyOn>(NotifyOn::Read), tag_);

            return tag_;
        }

        template <class U>
        void push(U&& u)
        {
            Queue<T>::push(std::forward<U>(u));

            if (isBound())
            {
                uint64_t val = 1;
                TRY(write(event_fd, &val, sizeof val));
            }
        }

        Entry* pop() override
        {
            auto ret = Queue<T>::pop();

            if (isBound())
            {
                uint64_t val;
                for (;;)
                {
                    ssize_t bytes = read(event_fd, &val, sizeof val);
                    if (bytes == -1)
                    {
                        if (errno == EAGAIN || errno == EWOULDBLOCK)
                            break;
                        else
                        {
                            // TODO
                        }
                    }
                }
            }

            return ret;
        }

        Polling::Tag tag() const
        {
            if (!isBound())
                throw std::runtime_error("Can not retrieve tag of an unbound mailbox");

            return Polling::Tag(event_fd);
        }

        void unbind(Polling::Epoll& poller)
        {
            if (event_fd == -1)
            {
                throw std::runtime_error("The mailbox is not bound");
            }

            poller.removeFd(event_fd);
            close(event_fd), event_fd = -1;
        }

    private:
        int event_fd;
    };

    // A Multi-Producer Multi-Consumer bounded queue
    // taken from
    // http://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue
    template <typename T, size_t Size>
    class MPMCQueue
    {

        static_assert(Size >= 2 && ((Size & (Size - 1)) == 0),
                      "The size must be a power of 2");
        static constexpr size_t Mask = Size - 1;

    public:
        MPMCQueue(const MPMCQueue& other) = delete;
        MPMCQueue& operator=(const MPMCQueue& other) = delete;

        /*
   * Note that you should not move a queue. This is somehow needed for gcc 4.7,
   * otherwise the client won't compile
   * @Investigate why
   */
        MPMCQueue(MPMCQueue&& other) { *this = std::move(other); }

        MPMCQueue& operator=(MPMCQueue&& other)
        {
            for (size_t i = 0; i < Size; ++i)
            {
                cells_[i].sequence.store(
                    other.cells_[i].sequence.load(std::memory_order_relaxed),
                    std::memory_order_relaxed);
                cells_[i].data = std::move(other.cells_[i].data);
            }

            enqueueIndex.store(other.enqueueIndex.load(), std::memory_order_relaxed);
            dequeueIndex.store(other.enqueueIndex.load(), std::memory_order_relaxed);
            return *this;
        }

        MPMCQueue()
            : cells_()
            , enqueueIndex()
            , dequeueIndex()
        {
            for (size_t i = 0; i < Size; ++i)
            {
                cells_[i].sequence.store(i, std::memory_order_relaxed);
            }

            enqueueIndex.store(0, std::memory_order_relaxed);
            dequeueIndex.store(0, std::memory_order_relaxed);
        }

        template <typename U>
        bool enqueue(U&& data)
        {
            Cell* target;
            size_t index = enqueueIndex.load(std::memory_order_relaxed);
            for (;;)
            {
                target     = cell(index);
                size_t seq = target->sequence.load(std::memory_order_acquire);
                auto diff  = static_cast<std::intptr_t>(seq) - static_cast<std::intptr_t>(index);
                if (diff == 0)
                {
                    if (enqueueIndex.compare_exchange_weak(index, index + 1,
                                                           std::memory_order_relaxed))
                        break;
                }

                else if (diff < 0)
                    return false;
                else
                {
                    index = enqueueIndex.load(std::memory_order_relaxed);
                }
            }
            target->data = std::forward<U>(data);
            target->sequence.store(index + 1, std::memory_order_release);
            return true;
        }

        bool dequeue(T& data)
        {
            Cell* target;
            size_t index = dequeueIndex.load(std::memory_order_relaxed);
            for (;;)
            {
                target     = cell(index);
                size_t seq = target->sequence.load(std::memory_order_acquire);
                auto diff  = static_cast<std::intptr_t>(seq) - static_cast<std::intptr_t>(index + 1);
                if (diff == 0)
                {
                    if (dequeueIndex.compare_exchange_weak(index, index + 1,
                                                           std::memory_order_relaxed))
                        break;
                }
                else if (diff < 0)
                    return false;
                else
                {
                    index = dequeueIndex.load(std::memory_order_relaxed);
                }
            }
            data = target->data;
            target->sequence.store(index + Mask + 1, std::memory_order_release);
            return true;
        }

    private:
        struct Cell
        {
            Cell()
                : sequence()
                , data()
            { }
            std::atomic<size_t> sequence;
            T data;
        };

        size_t cellIndex(size_t index) const { return index & Mask; }

        Cell* cell(size_t index) { return &cells_[cellIndex(index)]; }

        std::array<Cell, Size> cells_;

        cacheline_pad_t pad0;
        std::atomic<size_t> enqueueIndex;

        cacheline_pad_t pad1;
        std::atomic<size_t> dequeueIndex;
    };

} // namespace Pistache
