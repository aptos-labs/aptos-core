/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* async.h
   Mathieu Stefani, 05 novembre 2015

  This header brings a Promise<T> class inspired by the Promises/A+
  specification for asynchronous operations
*/

#pragma once

#include <pistache/typeid.h>

#include <atomic>
#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <type_traits>
#include <typeinfo>
#include <vector>

namespace Pistache::Async
{

    class Error : public std::runtime_error
    {
    public:
        explicit Error(const char* what)
            : std::runtime_error(what)
        { }
        explicit Error(const std::string& what)
            : std::runtime_error(what)
        { }
    };

    class BadType : public Error
    {
    public:
        explicit BadType(TypeId id)
            : Error("Argument type can not be used to resolve the promise "
                    " (TypeId does not match)")
            , id_(std::move(id))
        { }

        TypeId typeId() const { return id_; }

    private:
        TypeId id_;
    };

    class BadAnyCast : public std::bad_cast
    {
    public:
        const char* what() const noexcept override { return "Bad any cast"; }
        ~BadAnyCast() override = default;
    };

    enum class State { Pending,
                       Fulfilled,
                       Rejected };

    template <typename T>
    class Promise;

    class PromiseBase
    {
    public:
        virtual ~PromiseBase()           = default;
        virtual bool isPending() const   = 0;
        virtual bool isFulfilled() const = 0;
        virtual bool isRejected() const  = 0;

        bool isSettled() const { return isFulfilled() || isRejected(); }
    };

    namespace detail
    {
        template <typename Func, typename T>
        struct IsCallable
        {

            template <typename U>
            static auto test(U*)
                -> decltype(std::declval<Func>()(std::declval<U>()), std::true_type());

            template <typename U>
            static auto test(...) -> std::false_type;

            static constexpr bool value = std::is_same<decltype(test<T>(0)), std::true_type>::value;
        };

        template <typename Func>
        struct IsMoveCallable : public IsMoveCallable<decltype(&Func::operator())>
        { };

        template <typename R, typename Class, typename Arg>
        struct IsMoveCallable<R (Class::*)(Arg) const>
            : public std::is_rvalue_reference<Arg>
        { };

        template <typename Func, typename Arg>
        typename std::conditional<IsMoveCallable<Func>::value, Arg&&,
                                  const Arg&>::type
        tryMove(Arg& arg)
        {
            return std::move(arg);
        }

        template <typename Func>
        struct FunctionTrait : public FunctionTrait<decltype(&Func::operator())>
        { };

        template <typename R, typename Class, typename... Args>
        struct FunctionTrait<R (Class::*)(Args...) const>
        {
            typedef R ReturnType;

            static constexpr size_t ArgsCount = sizeof...(Args);
        };

        template <typename R, typename Class, typename... Args>
        struct FunctionTrait<R (Class::*)(Args...)>
        {
            typedef R ReturnType;

            static constexpr size_t ArgsCount = sizeof...(Args);
        };

        template <typename T>
        struct RemovePromise
        {
            typedef T Type;
        };

        template <typename T>
        struct RemovePromise<Promise<T>>
        {
            typedef T Type;
        };

        template <size_t N, typename... T>
        struct nth_element;

        template <typename Head, typename... Tail>
        struct nth_element<0, Head, Tail...>
        {
            typedef Head type;
        };

        template <size_t N, typename Head, typename... Tail>
        struct nth_element<N, Head, Tail...>
        {
            typedef typename nth_element<N - 1, Tail...>::type type;
        };

    } // namespace detail

    namespace Private
    {

        struct InternalRethrow
        {
            explicit InternalRethrow(std::exception_ptr _exc)
                : exc(std::move(_exc))
            { }

            std::exception_ptr exc;
        };

        struct IgnoreException
        {
            void operator()(std::exception_ptr) const { }
        };

        struct NoExcept
        {
            void operator()(std::exception_ptr) const { std::terminate(); }
        };

        struct Throw
        {
            void operator()(std::exception_ptr exc) const
            {
                throw InternalRethrow(std::move(exc));
            }
        };

        struct Core;

        class Request
        {
        public:
            virtual void resolve(const std::shared_ptr<Core>& core) = 0;
            virtual void reject(const std::shared_ptr<Core>& core)  = 0;
            virtual ~Request()                                      = default;
        };

        struct Core
        {
            Core(State _state, TypeId _id)
                : allocated(false)
                , state(_state)
                , exc()
                , mtx()
                , requests()
                , id(_id)
            { }

            bool allocated;
            std::atomic<State> state;
            std::exception_ptr exc;

            /*
   * We need this lock because a Promise might be resolved or rejected from a
   * thread A while a continuation to the same Promise (Core) might be attached
   * at the same from a thread B. If that's the case, then we need to serialize
   * operations so that we avoid a race-condition.
   *
   * Since we have a lock, we have a blocking progress guarantee but I don't
   * expect this to be a major bottleneck as I don't expect major contention on
   * the lock If it ends up being a bottlenick, try @improving it by
   * experimenting with a lock-free scheme
   */
            std::mutex mtx;
            std::vector<std::shared_ptr<Request>> requests;
            TypeId id;

            virtual void* memory() = 0;

            virtual bool isVoid() const = 0;

            template <typename T, typename... Args>
            void construct(Args&&... args)
            {
                if (isVoid())
                    throw Error("Can not construct a void core");

                if (id != TypeId::of<T>())
                {
                    throw BadType(id);
                }

                void* mem = memory();

                if (allocated)
                {
                    reinterpret_cast<T*>(mem)->~T();
                    allocated = false;
                }

                new (mem) T(std::forward<Args>(args)...);
                allocated = true;
                state     = State::Fulfilled;
            }

            virtual ~Core() = default;
        };

        template <typename T>
        struct CoreT : public Core
        {
            CoreT()
                : Core(State::Pending, TypeId::of<T>())
                , storage()
            { }

            ~CoreT() override
            {
                if (allocated)
                {
                    reinterpret_cast<T*>(&storage)->~T();
                    allocated = false;
                }
            }

            template <class Other>
            struct Rebind
            {
                typedef CoreT<Other> Type;
            };

            T& value()
            {
                if (state != State::Fulfilled)
                    throw Error("Attempted to take the value of a not fulfilled promise");

                return *reinterpret_cast<T*>(&storage);
            }

            bool isVoid() const override { return false; }

        protected:
            void* memory() override { return &storage; }

        private:
            typedef typename std::aligned_storage<sizeof(T), alignof(T)>::type Storage;
            Storage storage;
        };

        template <>
        struct CoreT<void> : public Core
        {
            CoreT()
                : Core(State::Pending, TypeId::of<void>())
            { }

            bool isVoid() const override { return true; }

        protected:
            void* memory() override { return nullptr; }
        };

        template <typename T>
        struct Continuable : public Request
        {
            explicit Continuable(const std::shared_ptr<Core>& chain)
                : resolveCount_(0)
                , rejectCount_(0)
                , chain_(chain)
            { }

            void resolve(const std::shared_ptr<Core>& core) override
            {
                if (resolveCount_ >= 1)
                    return; // TODO is this the right thing?
                        // throw Error("Resolve must not be called more than once");

                ++resolveCount_;
                doResolve(coreCast(core));
            }

            void reject(const std::shared_ptr<Core>& core) override
            {
                if (rejectCount_ >= 1)
                    return; // TODO is this the right thing?
                        // throw Error("Reject must not be called more than once");

                ++rejectCount_;
                try
                {
                    doReject(coreCast(core));
                }
                catch (const InternalRethrow& e)
                {
                    chain_->exc   = e.exc;
                    chain_->state = State::Rejected;
                    for (const auto& req : chain_->requests)
                    {
                        req->reject(chain_);
                    }
                }
            }

            std::shared_ptr<CoreT<T>> coreCast(const std::shared_ptr<Core>& core) const
            {
                return std::static_pointer_cast<CoreT<T>>(core);
            }

            virtual void doResolve(const std::shared_ptr<CoreT<T>>& core) = 0;
            virtual void doReject(const std::shared_ptr<CoreT<T>>& core)  = 0;

            ~Continuable() override = default;

            size_t resolveCount_;
            size_t rejectCount_;
            std::shared_ptr<Core> chain_;
        };

        namespace impl
        {

            template <typename T, typename Resolve, typename Reject, typename U>
            struct Continuation;

            template <typename T, typename Resolve, typename Reject, typename Res,
                      typename Cls, typename... Args>
            struct Continuation<T, Resolve, Reject, Res (Cls::*)(Args...) const>
                : public Continuation<T, Resolve, Reject, Res(Args...)>
            {
                typedef Continuation<T, Resolve, Reject, Res(Args...)> Base;

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Base(chain, std::move(resolve), std::move(reject))
                { }
            };

            template <typename T, typename Resolve, typename Reject, typename Res,
                      typename Cls, typename... Args>
            struct Continuation<T, Resolve, Reject, Res (Cls::*)(Args...)>
                : public Continuation<T, Resolve, Reject, Res(Args...)>
            {
                typedef Continuation<T, Resolve, Reject, Res(Args...)> Base;

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Base(chain, std::move(resolve), std::move(reject))
                { }
            };

            // General specialization
            template <typename T, typename Resolve, typename Reject, typename Res,
                      typename... Args>
            struct Continuation<T, Resolve, Reject, Res(Args...)> : public Continuable<T>
            {

                static_assert(sizeof...(Args) == 1,
                              "A continuation should only take one single argument");

                typedef typename detail::nth_element<0, Args...>::type Arg;

                static_assert(std::is_same<T, Arg>::value || std::is_convertible<T, Arg>::value,
                              "Incompatible types detected");

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<T>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                void doResolve(const std::shared_ptr<CoreT<T>>& core) override
                {
                    finishResolve(resolve_(detail::tryMove<Resolve>(core->value())));
                }

                void doReject(const std::shared_ptr<CoreT<T>>& core) override
                {
                    reject_(core->exc);
                    for (const auto& req : this->chain_->requests)
                    {
                        req->reject(this->chain_);
                    }
                }

                template <typename Ret>
                void finishResolve(Ret&& ret) const
                {
                    typedef typename std::decay<Ret>::type CleanRet;
                    this->chain_->template construct<CleanRet>(std::forward<Ret>(ret));
                    for (const auto& req : this->chain_->requests)
                    {
                        req->resolve(this->chain_);
                    }
                }

                Resolve resolve_;
                Reject reject_;
            };

            // Specialization for a void-Promise
            template <typename Resolve, typename Reject, typename Res, typename... Args>
            struct Continuation<void, Resolve, Reject, Res(Args...)>
                : public Continuable<void>
            {

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<void>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                static_assert(sizeof...(Args) == 0,
                              "Can not attach a non-void continuation to a void-Promise");

                void doResolve(const std::shared_ptr<CoreT<void>>& /*core*/) override
                {
                    finishResolve(resolve_());
                }

                void doReject(const std::shared_ptr<CoreT<void>>& core) override
                {
                    reject_(core->exc);
                    for (const auto& req : this->chain_->requests)
                    {
                        req->reject(this->chain_);
                    }
                }

                template <typename Ret>
                void finishResolve(Ret&& ret) const
                {
                    typedef typename std::remove_reference<Ret>::type CleanRet;
                    this->chain_->template construct<CleanRet>(std::forward<Ret>(ret));
                    for (const auto& req : this->chain_->requests)
                    {
                        req->resolve(this->chain_);
                    }
                }

                Resolve resolve_;
                Reject reject_;
            };

            // Specialization for a callback returning void
            template <typename T, typename Resolve, typename Reject, typename... Args>
            struct Continuation<T, Resolve, Reject, void(Args...)> : public Continuable<T>
            {

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<T>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                static_assert(sizeof...(Args) == 1,
                              "A continuation should only take one single argument");

                typedef typename detail::nth_element<0, Args...>::type Arg;

                static_assert(std::is_same<T, Arg>::value || std::is_convertible<T, Arg>::value,
                              "Incompatible types detected");

                void doResolve(const std::shared_ptr<CoreT<T>>& core) override
                {
                    resolve_(core->value());
                }

                void doReject(const std::shared_ptr<CoreT<T>>& core) override
                {
                    reject_(core->exc);
                }

                Resolve resolve_;
                Reject reject_;
            };

            // Specialization for a void-Promise on a callback returning void
            template <typename Resolve, typename Reject, typename... Args>
            struct Continuation<void, Resolve, Reject, void(Args...)>
                : public Continuable<void>
            {

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<void>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                static_assert(sizeof...(Args) == 0,
                              "Can not attach a non-void continuation to a void-Promise");

                void doResolve(const std::shared_ptr<CoreT<void>>& /*core*/) override
                {
                    resolve_();
                }

                void doReject(const std::shared_ptr<CoreT<void>>& core) override
                {
                    reject_(core->exc);
                }

                Resolve resolve_;
                Reject reject_;
            };

            // Specialization for a callback returning a Promise
            template <typename T, typename Resolve, typename Reject, typename U,
                      typename... Args>
            struct Continuation<T, Resolve, Reject, Promise<U>(Args...)>
                : public Continuable<T>
            {

                static_assert(sizeof...(Args) == 1,
                              "A continuation should only take one single argument");

                typedef typename detail::nth_element<0, Args...>::type Arg;

                static_assert(std::is_same<T, Arg>::value || std::is_convertible<T, Arg>::value,
                              "Incompatible types detected");

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<T>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                void doResolve(const std::shared_ptr<CoreT<T>>& core) override
                {
                    auto promise = resolve_(detail::tryMove<Resolve>(core->value()));
                    finishResolve(promise);
                }

                void doReject(const std::shared_ptr<CoreT<T>>& core) override
                {
                    reject_(core->exc);
                    for (const auto& req : core->requests)
                    {
                        req->reject(core);
                    }
                }

                template <typename PromiseType>
                struct Chainer
                {
                    explicit Chainer(const std::shared_ptr<Private::Core>& core)
                        : chainCore(core)
                    { }

                    void operator()(const PromiseType& val)
                    {
                        chainCore->construct<PromiseType>(val);
                        for (const auto& req : chainCore->requests)
                        {
                            req->resolve(chainCore);
                        }
                    }

                    std::shared_ptr<Core> chainCore;
                };

                template <typename Promise,
                          typename Type = typename detail::RemovePromise<Promise>::Type>
                Chainer<Type> makeChainer(const Promise&)
                {
                    return Chainer<Type>(this->chain_);
                }

                template <typename P>
                void finishResolve(P& promise)
                {
                    auto chainer                = makeChainer(promise);
                    std::weak_ptr<Core> weakPtr = this->chain_;
                    promise.then(std::move(chainer), [weakPtr](std::exception_ptr exc) {
                        if (auto core = weakPtr.lock())
                        {
                            core->exc   = std::move(exc);
                            core->state = State::Rejected;

                            for (const auto& req : core->requests)
                            {
                                req->reject(core);
                            }
                        }
                    });
                }

                Resolve resolve_;
                Reject reject_;
            };

            // Specialization for a void callback returning a Promise
            template <typename Resolve, typename Reject, typename U, typename... Args>
            struct Continuation<void, Resolve, Reject, Promise<U>(Args...)>
                : public Continuable<void>
            {

                static_assert(sizeof...(Args) == 0,
                              "Can not attach a non-void continuation to a void-Promise");

                Continuation(const std::shared_ptr<Core>& chain, Resolve resolve,
                             Reject reject)
                    : Continuable<void>(chain)
                    , resolve_(resolve)
                    , reject_(reject)
                { }

                void doResolve(const std::shared_ptr<CoreT<void>>& /*core*/) override
                {
                    auto promise = resolve_();
                    finishResolve(promise);
                }

                void doReject(const std::shared_ptr<CoreT<void>>& core) override
                {
                    reject_(core->exc);
                    for (const auto& req : core->requests)
                    {
                        req->reject(core);
                    }
                }

                template <typename PromiseType, typename Dummy = void>
                struct Chainer
                {
                    explicit Chainer(const std::shared_ptr<Private::Core>& core)
                        : chainCore(core)
                    { }

                    void operator()(const PromiseType& val)
                    {
                        chainCore->construct<PromiseType>(val);
                        for (const auto& req : chainCore->requests)
                        {
                            req->resolve(chainCore);
                        }
                    }

                    std::shared_ptr<Core> chainCore;
                };

                template <typename Dummy>
                struct Chainer<void, Dummy>
                {
                    explicit Chainer(const std::shared_ptr<Private::Core>& core)
                        : chainCore(core)
                    { }

                    void operator()()
                    {
                        auto core   = this->chain_;
                        core->state = State::Fulfilled;

                        for (const auto& req : chainCore->requests)
                        {
                            req->resolve(chainCore);
                        }
                    }

                    std::shared_ptr<Core> chainCore;
                };

                template <typename Promise,
                          typename Type = typename detail::RemovePromise<Promise>::Type>
                Chainer<Type> makeChainer(const Promise&)
                {
                    return Chainer<Type>(this->chain_);
                }

                template <typename P>
                void finishResolve(P& promise)
                {
                    auto chainer = makeChainer(promise);
                    promise.then(std::move(chainer), [=](std::exception_ptr exc) {
                        auto core   = this->chain_;
                        core->exc   = std::move(exc);
                        core->state = State::Rejected;

                        for (const auto& req : core->requests)
                        {
                            req->reject(core);
                        }
                    });
                }

                Resolve resolve_;
                Reject reject_;
            };

        } // namespace impl

        template <typename T, typename Resolve, typename Reject, typename Sig>
        struct Continuation : public impl::Continuation<T, Resolve, Reject,
                                                        decltype(&Sig::operator())>
        {

            typedef impl::Continuation<T, Resolve, Reject, decltype(&Sig::operator())>
                Base;

            Continuation(const std::shared_ptr<Core>& core, Resolve resolve,
                         Reject reject)
                : Base(core, std::move(resolve), std::move(reject))
            { }
        };

        template <typename T, typename Resolve, typename Reject, typename Res,
                  typename... Args>
        struct Continuation<T, Resolve, Reject, Res (*)(Args...)>
            : public impl::Continuation<T, Resolve, Reject, Res(Args...)>
        {
            typedef impl::Continuation<T, Resolve, Reject, Res(Args...)> Base;

            Continuation(const std::shared_ptr<Core>& core, Resolve resolve,
                         Reject reject)
                : Base(core, std::move(resolve), std::move(reject))
            { }
        };

        template <typename T, typename Resolve, typename Reject, typename Res,
                  typename Cls, typename... Args>
        struct Continuation<T, Resolve, Reject, Res (Cls::*)(Args...)>
            : public impl::Continuation<T, Resolve, Reject, Res(Args...)>
        {
            typedef impl::Continuation<T, Resolve, Reject, Res(Args...)> Base;

            Continuation(const std::shared_ptr<Core>& core, Resolve resolve,
                         Reject reject)
                : Base(core, std::move(resolve), std::move(reject))
            { }
        };

        template <typename T, typename Resolve, typename Reject, typename Res,
                  typename Cls, typename... Args>
        struct Continuation<T, Resolve, Reject, Res (Cls::*)(Args...) const>
            : public impl::Continuation<T, Resolve, Reject, Res(Args...)>
        {
            typedef impl::Continuation<T, Resolve, Reject, Res(Args...)> Base;

            Continuation(const std::shared_ptr<Core>& core, Resolve resolve,
                         Reject reject)
                : Base(core, std::move(resolve), std::move(reject))
            { }
        };

        template <typename T, typename Resolve, typename Reject, typename Res,
                  typename... Args>
        struct Continuation<T, Resolve, Reject, std::function<Res(Args...)>>
            : public impl::Continuation<T, Resolve, Reject, Res(Args...)>
        {
            typedef impl::Continuation<T, Resolve, Reject, Res(Args...)> Base;

            Continuation(const std::shared_ptr<Core>& core, Resolve resolve,
                         Reject reject)
                : Base(core, std::move(resolve), std::move(reject))
            { }
        };
    } // namespace Private

    class Resolver
    {
    public:
        explicit Resolver(const std::shared_ptr<Private::Core>& core)
            : core_(core)
        { }

        Resolver(const Resolver& other) = delete;
        Resolver& operator=(const Resolver& other) = delete;

        Resolver(Resolver&& other) = default;
        Resolver& operator=(Resolver&& other) = default;

        template <typename Arg>
        bool operator()(Arg&& arg) const
        {
            if (!core_)
                return false;

            typedef typename std::remove_reference<Arg>::type Type;

            if (core_->state != State::Pending)
                throw Error("Attempt to resolve a fulfilled promise");

            /* In a ideal world, this should be checked at compile-time rather
     * than runtime. However, since types are erased, this looks like
     * a difficult task
     */
            if (core_->isVoid())
            {
                throw Error("Attempt to resolve a void promise with arguments");
            }

            std::unique_lock<std::mutex> guard(core_->mtx);
            core_->construct<Type>(std::forward<Arg>(arg));

            for (const auto& req : core_->requests)
            {
                req->resolve(core_);
            }

            return true;
        }

        bool operator()() const
        {
            if (!core_)
                return false;

            if (core_->state != State::Pending)
                throw Error("Attempt to resolve a fulfilled promise");

            if (!core_->isVoid())
                throw Error("Attempt ro resolve a non-void promise with no argument");

            std::unique_lock<std::mutex> guard(core_->mtx);
            core_->state = State::Fulfilled;
            for (const auto& req : core_->requests)
            {
                req->resolve(core_);
            }

            return true;
        }

        void clear() { core_ = nullptr; }

        Resolver clone() { return Resolver(core_); }

    private:
        std::shared_ptr<Private::Core> core_;
    };

    class Rejection
    {
    public:
        explicit Rejection(const std::shared_ptr<Private::Core>& core)
            : core_(core)
        { }

        Rejection(const Rejection& other) = delete;
        Rejection& operator=(const Rejection& other) = delete;

        Rejection(Rejection&& other) = default;
        Rejection& operator=(Rejection&& other) = default;

        template <typename Exc>
        bool operator()(Exc exc) const
        {
            if (!core_)
                return false;

            if (core_->state != State::Pending)
                throw Error("Attempt to reject a fulfilled promise");

            std::unique_lock<std::mutex> guard(core_->mtx);
            core_->exc   = std::make_exception_ptr(exc);
            core_->state = State::Rejected;
            for (const auto& req : core_->requests)
            {
                req->reject(core_);
            }

            return true;
        }

        void clear() { core_ = nullptr; }

        Rejection clone() { return Rejection(core_); }

    private:
        std::shared_ptr<Private::Core> core_;
    };

    template <typename T>
    class Deferred
    {
    public:
        Deferred()
            : resolver(nullptr)
            , rejection(nullptr)
        { }

        Deferred(const Deferred& other) = delete;
        Deferred& operator=(const Deferred& other) = delete;

        Deferred(Deferred&& other) = default;
        Deferred& operator=(Deferred&& other) = default;

        Deferred(Resolver _resolver, Rejection _reject)
            : resolver(std::move(_resolver))
            , rejection(std::move(_reject))
        { }

        template <typename U>
        bool resolve(U&& arg)
        {
            typedef typename std::remove_reference<U>::type CleanU;

            static_assert(std::is_same<T, CleanU>::value || std::is_convertible<U, T>::value,
                          "Types mismatch");

            return resolver(std::forward<U>(arg));
        }

        template <typename... Args>
        void emplaceResolve(Args&&...) { }

        template <typename Exc>
        bool reject(Exc exc)
        {
            return rejection(std::move(exc));
        }

        void clear()
        {
            resolver.clear();
            rejection.clear();
        }

    private:
        Resolver resolver;
        Rejection rejection;
    };

    template <>
    class Deferred<void>
    {
    public:
        Deferred()
            : resolver(nullptr)
            , rejection(nullptr)
        { }

        Deferred(const Deferred& other) = delete;
        Deferred& operator=(const Deferred& other) = delete;

        Deferred(Deferred&& other) = default;
        Deferred& operator=(Deferred&& other) = default;

        Deferred(Resolver _resolver, Rejection _reject)
            : resolver(std::move(_resolver))
            , rejection(std::move(_reject))
        { }

        void resolve() { resolver(); }

        template <typename Exc>
        void reject(Exc _exc) { rejection(std::move(_exc)); }

    private:
        Resolver resolver;
        Rejection rejection;
    };

    static constexpr Private::IgnoreException IgnoreException {};
    static constexpr Private::NoExcept NoExcept {};
    static constexpr Private::Throw Throw {};

    namespace details
    {

        /*
 * Note that we could use std::result_of to SFINAE-out and dispatch to the right
 * call However, gcc 4.7 does not correctly support std::result_of for SFINAE
 * purposes, so we use a decltype SFINAE-expression instead.
 *
 * See http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2012/n3462.html and
 * https://gcc.gnu.org/bugzilla/show_bug.cgi?id=56283 for reference
 */
        template <typename T, typename Func>
        auto callAsync(Func func, Resolver& resolver, Rejection& rejection)
            -> decltype(std::declval<Func>()(resolver, rejection), void())
        {
            func(resolver, rejection);
        }

        template <typename T, typename Func>
        auto callAsync(Func func, Resolver& resolver, Rejection& rejection)
            -> decltype(std::declval<Func>()(Deferred<T>()), void())
        {
            func(Deferred<T>(std::move(resolver), std::move(rejection)));
        }
    } // namespace details

    template <typename T>
    class Promise : public PromiseBase
    {
    public:
        template <typename U>
        friend class Promise;

        typedef Private::CoreT<T> Core;

        template <typename Func>
        explicit Promise(Func func)
            : core_(std::make_shared<Core>())
            , resolver_(core_)
            , rejection_(core_)
        {
            details::callAsync<T>(func, resolver_, rejection_);
        }

        Promise(const Promise<T>& other) = delete;
        Promise& operator=(const Promise<T>& other) = delete;

        Promise(Promise<T>&& other) = default;
        Promise& operator=(Promise<T>&& other) = default;

        ~Promise() override = default;

        template <typename U>
        static Promise<T> resolved(U&& value)
        {
            static_assert(!std::is_void<T>::value,
                          "Can not resolve a void promise with parameters");
            static_assert(std::is_same<T, U>::value || std::is_convertible<U, T>::value,
                          "Incompatible value type");

            auto core = std::make_shared<Core>();
            core->template construct<T>(std::forward<U>(value));
            return Promise<T>(std::move(core));
        }

        static Promise<void> resolved()
        {
            static_assert(std::is_void<T>::value,
                          "Resolving a non-void promise requires parameters");

            auto core   = std::make_shared<Core>();
            core->state = State::Fulfilled;
            return Promise<T>(std::move(core));
        }

        template <typename Exc>
        static Promise<T> rejected(Exc exc)
        {
            auto core   = std::make_shared<Core>();
            core->exc   = std::make_exception_ptr(exc);
            core->state = State::Rejected;
            return Promise<T>(std::move(core));
        }

        bool isPending() const override { return core_->state == State::Pending; }
        bool isFulfilled() const override { return core_->state == State::Fulfilled; }
        bool isRejected() const override { return core_->state == State::Rejected; }

        template <typename ResolveFunc, typename RejectFunc>
        auto then(ResolveFunc resolveFunc, RejectFunc rejectFunc)
            -> Promise<typename detail::RemovePromise<
                typename detail::FunctionTrait<ResolveFunc>::ReturnType>::Type>
        {

            typedef typename detail::RemovePromise<
                typename detail::FunctionTrait<ResolveFunc>::ReturnType>::Type RetType;

            Promise<RetType> promise;

            typedef Private::Continuation<T, ResolveFunc, RejectFunc, ResolveFunc>
                Continuation;
            std::shared_ptr<Private::Request> req = std::make_shared<Continuation>(promise.core_, resolveFunc, rejectFunc);

            std::unique_lock<std::mutex> guard(core_->mtx);
            if (isFulfilled())
            {
                req->resolve(core_);
            }
            else if (isRejected())
            {
                req->reject(core_);
            }

            core_->requests.push_back(req);

            return promise;
        }

    private:
        Promise()
            : core_(std::make_shared<Core>())
            , resolver_(core_)
            , rejection_(core_)
        { }

        explicit Promise(std::shared_ptr<Core>&& core)
            : core_(core)
            , resolver_(core_)
            , rejection_(core_)
        { }

        std::shared_ptr<Core> core_;
        Resolver resolver_;
        Rejection rejection_;
    };

    template <typename T>
    class Barrier
    {
    public:
        explicit Barrier(Promise<T>& promise)
            : promise_(promise)
        { }

        void wait()
        {
            if (promise_.isFulfilled() || promise_.isRejected())
                return;

            promise_.then(
                [&](const T&) mutable {
                    std::unique_lock<std::mutex> guard(mtx);
                    cv.notify_one();
                },
                [&](std::exception_ptr) mutable {
                    std::unique_lock<std::mutex> guard(mtx);
                    cv.notify_one();
                });

            std::unique_lock<std::mutex> guard(mtx);
            cv.wait(guard,
                    [&] { return promise_.isFulfilled() || promise_.isRejected(); });
        }

        template <class Rep, class Period>
        std::cv_status wait_for(const std::chrono::duration<Rep, Period>& period)
        {
            if (promise_.isFulfilled() || promise_.isRejected())
                return std::cv_status::no_timeout;

            promise_.then(
                [&](const T&) mutable {
                    std::unique_lock<std::mutex> guard(mtx);
                    cv.notify_one();
                },
                [&](std::exception_ptr) mutable {
                    std::unique_lock<std::mutex> guard(mtx);
                    cv.notify_one();
                });

            std::unique_lock<std::mutex> guard(mtx);
            return cv.wait_for(guard, period);
        }

    private:
        Promise<T>& promise_;
        mutable std::mutex mtx;
        std::condition_variable cv;
    };

    namespace Impl
    {
        struct Any;
    }

    class Any
    {
    public:
        friend struct Impl::Any;

        Any(const Any& other) = default;
        Any& operator=(const Any& other) = default;

        Any(Any&& other) = default;
        Any& operator=(Any&& other) = default;

        template <typename T>
        bool is() const { return core_->id == TypeId::of<T>(); }

        template <typename T>
        T cast() const
        {
            if (!is<T>())
                throw BadAnyCast();

            auto core = std::static_pointer_cast<Private::CoreT<T>>(core_);
            return core->value();
        }

    private:
        explicit Any(const std::shared_ptr<Private::Core>& core)
            : core_(core)
        { }
        std::shared_ptr<Private::Core> core_;
    };

    namespace Impl
    {

        /* Instead of duplicating the code between whenAll and whenAny functions, the
 * main implementation is in the When class below and we configure the class
 * with a policy instead,  depending if we are executing an "all" or "any"
 * operation, how cool is that ?
 */
        struct All
        {

            struct Data
            {
                Data(const size_t _total, Resolver _resolver, Rejection _rejection)
                    : total(_total)
                    , resolved(0)
                    , rejected(false)
                    , mtx()
                    , resolve(std::move(_resolver))
                    , reject(std::move(_rejection))
                { }

                const size_t total;
                size_t resolved;
                bool rejected;
                std::mutex mtx;

                Resolver resolve;
                Rejection reject;
            };

            template <size_t Index, typename T, typename Data>
            static void resolveT(const T& val, Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                if (data->rejected)
                    return;

                // @Check thread-safety of std::get ?
                std::get<Index>(data->results) = val;
                data->resolved++;

                if (data->resolved == data->total)
                {
                    data->resolve(data->results);
                }
            }

            template <typename Data>
            static void resolveVoid(Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                if (data->rejected)
                    return;

                data->resolved++;

                if (data->resolved == data->total)
                {
                    data->resolve(data->results);
                }
            }

            template <typename Data>
            static void reject(std::exception_ptr exc, Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                data->rejected = true;
                data->reject(exc);
            }
        };

        struct Any
        {

            struct Data
            {
                Data(size_t, Resolver resolver, Rejection rejection)
                    : done(false)
                    , mtx()
                    , resolve(std::move(resolver))
                    , reject(std::move(rejection))
                { }

                bool done;
                std::mutex mtx;

                Resolver resolve;
                Rejection reject;
            };

            template <size_t Index, typename T, typename Data>
            static void resolveT(const T& val, Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                if (data->done)
                    return;

                // Instead of allocating a new core, ideally we could share the same core as
                // the relevant promise but we do not have access to the promise here is so
                // meh
                auto core = std::make_shared<Private::CoreT<T>>();
                core->template construct<T>(val);
                data->resolve(Async::Any(core));

                data->done = true;
            }

            template <typename Data>
            static void resolveVoid(Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                if (data->done)
                    return;

                auto core = std::make_shared<Private::CoreT<void>>();
                data->resolve(Async::Any(core));

                data->done = true;
            }

            template <typename Data>
            static void reject(std::exception_ptr exc, Data& data)
            {
                std::lock_guard<std::mutex> guard(data->mtx);

                data->done = true;
                data->reject(exc);
            }
        };

        template <typename ContinuationPolicy>
        struct When
        {
            When(Resolver resolver, Rejection rejection)
                : resolve(std::move(resolver))
                , reject(std::move(rejection))
            { }

            template <typename... Args>
            void operator()(Args&&... args)
            {
                whenArgs(std::forward<Args>(args)...);
            }

        private:
            template <typename T, size_t Index, typename Data>
            struct WhenContinuation
            {
                explicit WhenContinuation(Data _data)
                    : data(std::move(_data))
                { }

                void operator()(const T& val) const
                {
                    ContinuationPolicy::template resolveT<Index>(val, data);
                }

                Data data;
            };

            template <size_t Index, typename Data>
            struct WhenContinuation<void, Index, Data>
            {
                explicit WhenContinuation(Data _data)
                    : data(std::move(_data))
                { }

                void operator()() const { ContinuationPolicy::resolveVoid(data); }

                Data data;
            };

            template <typename T, size_t Index, typename Data>
            WhenContinuation<T, Index, Data> makeContinuation(const Data& data)
            {
                return WhenContinuation<T, Index, Data>(data);
            }

            template <size_t Index, typename Data, typename T>
            void when(const Data& data, Promise<T>& promise)
            {
                promise.then(makeContinuation<T, Index>(data), [=](std::exception_ptr ptr) {
                    ContinuationPolicy::reject(std::move(ptr), data);
                });
            }

            template <size_t Index, typename Data, typename T>
            void when(const Data& data, T&& arg)
            {
                typedef typename std::remove_reference<T>::type CleanT;
                auto promise = Promise<CleanT>::resolved(std::forward<T>(arg));
                when<Index>(data, promise);
            }

            template <typename... Args>
            void whenArgs(Args&&... args)
            {
                typedef std::tuple<typename detail::RemovePromise<
                    typename std::remove_reference<Args>::type>::Type...>
                    Results;
                /* We need to keep the results alive until the last promise
     * finishes its execution
     */

                /* See the trick here ? Basically, we only have access to the real type of
     * the results in this function. The policy classes do not have access to
     * the full type (std::tuple), but, instead, take a generic template data
     * type as a parameter. They only need to know that results is a tuple, they
     * do not need to know the real type of the results.
     *
     * This is some sort of compile-time template type-erasing, hue
     */
                struct Data : public ContinuationPolicy::Data
                {
                    Data(size_t total, Resolver resolver, Rejection rejection)
                        : ContinuationPolicy::Data(total, std::move(resolver),
                                                   std::move(rejection))
                    { }

                    Results results;
                };

                auto data = std::make_shared<Data>(sizeof...(Args), std::move(resolve),
                                                   std::move(reject));
                whenArgs<0>(data, std::forward<Args>(args)...);
            }

            template <size_t Index, typename Data, typename Head, typename... Rest>
            void whenArgs(const Data& data, Head&& head, Rest&&... rest)
            {
                when<Index>(data, std::forward<Head>(head));
                whenArgs<Index + 1>(data, std::forward<Rest>(rest)...);
            }

            template <size_t Index, typename Data, typename Head>
            void whenArgs(const Data& data, Head&& head)
            {
                when<Index>(data, std::forward<Head>(head));
            }

            Resolver resolve;
            Rejection reject;
        };

        template <typename T, typename Results>
        struct WhenAllRange
        {
            WhenAllRange(Resolver _resolve, Rejection _reject)
                : resolve(std::move(_resolve))
                , reject(std::move(_reject))
            { }

            template <typename Iterator>
            void operator()(Iterator first, Iterator last)
            {
                auto data = std::make_shared<DataT<T>>(
                    static_cast<size_t>(std::distance(first, last)), std::move(resolve),
                    std::move(reject));

                size_t index = 0;
                for (auto it = first; it != last; ++it)
                {

                    WhenContinuation<T> cont(data, index);

                    it->then(std::move(cont), [=](std::exception_ptr ptr) {
                        std::lock_guard<std::mutex> guard(data->mtx);

                        if (data->rejected)
                            return;

                        data->rejected = true;
                        data->reject(std::move(ptr));
                    });

                    ++index;
                }
            }

        private:
            struct Data
            {
                Data(size_t _total, Resolver _resolver, Rejection _rejection)
                    : total(_total)
                    , resolved(0)
                    , rejected(false)
                    , mtx()
                    , resolve(std::move(_resolver))
                    , reject(std::move(_rejection))
                { }

                const size_t total;
                size_t resolved;
                bool rejected;
                std::mutex mtx;

                Resolver resolve;
                Rejection reject;
            };

            /* Ok so apparently I can not fully specialize a template structure
   * here, so you know what, compiler ? Take that Dummy type and leave
   * me alone
   */
            template <typename ValueType, typename Dummy = void>
            struct DataT : public Data
            {
                DataT(size_t total, Resolver resolver, Rejection rejection)
                    : Data(total, std::move(resolver), std::move(rejection))
                {
                    results.resize(total);
                }

                Results results;
            };

            /* For a vector of void promises, we do not have any results, that's
   * why we need a distinct specialization for the void case
   */
            template <typename Dummy>
            struct DataT<void, Dummy> : public Data
            {
                DataT(size_t total, Resolver resolver, Rejection rejection)
                    : Data(total, std::move(resolver), std::move(rejection))
                { }
            };

            template <typename ValueType, typename Dummy = void>
            struct WhenContinuation
            {
                using D = std::shared_ptr<DataT<ValueType>>;

                WhenContinuation(const D& _data, size_t _index)
                    : data(_data)
                    , index(_index)
                { }

                void operator()(const ValueType& val) const
                {
                    std::lock_guard<std::mutex> guard(data->mtx);

                    if (data->rejected)
                        return;

                    data->results[index] = val;
                    data->resolved++;
                    if (data->resolved == data->total)
                    {
                        data->resolve(data->results);
                    }
                }

                D data;
                size_t index;
            };

            template <typename Dummy>
            struct WhenContinuation<void, Dummy>
            {
                using D = std::shared_ptr<DataT<void>>;

                WhenContinuation(const D& _data, size_t)
                    : data(_data)
                { }

                void operator()() const
                {
                    std::lock_guard<std::mutex> guard(data->mtx);

                    if (data->rejected)
                        return;

                    data->resolved++;
                    if (data->resolved == data->total)
                    {
                        data->resolve();
                    }
                }

                D data;
            };

            Resolver resolve;
            Rejection reject;
        };

    } // namespace Impl

    template <typename... Args,
              typename Results = std::tuple<typename detail::RemovePromise<
                  typename std::remove_reference<Args>::type>::Type...>>
    Promise<Results> whenAll(Args&&... args)
    {
        // As ugly as it looks, this is needed to bypass a bug of gcc < 4.9
        // whereby template parameters pack inside a lambda expression are not
        // captured correctly and can not be expanded inside the lambda.
        Resolver* resolve;
        Rejection* reject;

        Promise<Results> promise([&](Resolver& resolver, Rejection& rejection) {
            resolve = &resolver;
            reject  = &rejection;
        });

        Impl::When<Impl::All> impl(std::move(*resolve), std::move(*reject));
        // So we capture everything we need inside the lambda and then call the
        // implementation and expand the parameters pack here
        impl(std::forward<Args>(args)...);

        return promise;
    }

    template <typename... Args>
    Promise<Any> whenAny(Args&&... args)
    {
        // Same trick as above;
        Resolver* resolve;
        Rejection* reject;

        Promise<Any> promise([&](Resolver& resolver, Rejection& rejection) {
            resolve = &resolver;
            reject  = &rejection;
        });

        Impl::When<Impl::Any> impl(std::move(*resolve), std::move(*reject));
        impl(std::forward<Args>(args)...);
        return promise;
    }

    template <typename Iterator,
              typename ValueType = typename detail::RemovePromise<
                  typename std::iterator_traits<Iterator>::value_type>::Type,
              typename Results =
                  typename std::conditional<std::is_same<void, ValueType>::value,
                                            void, std::vector<ValueType>>::type>
    Promise<Results> whenAll(Iterator first, Iterator last)
    {
        /* @Security, assert that last >= first */

        return Promise<Results>([=](Resolver& resolve, Rejection& rejection) {
            Impl::WhenAllRange<ValueType, Results> impl(std::move(resolve),
                                                        std::move(rejection));

            impl(first, last);
        });
    }

} // namespace Pistache::Async
