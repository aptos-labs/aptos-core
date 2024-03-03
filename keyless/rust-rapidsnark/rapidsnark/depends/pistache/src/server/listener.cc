/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* listener.cc
   Mathieu Stefani, 12 August 2015

*/

#include <pistache/common.h>
#include <pistache/errors.h>
#include <pistache/listener.h>
#include <pistache/os.h>
#include <pistache/peer.h>
#include <pistache/ssl_wrappers.h>
#include <pistache/transport.h>

#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <sys/timerfd.h>
#include <sys/types.h>

#include <chrono>
#include <memory>
#include <vector>

#include <cerrno>
#include <signal.h>

#ifdef PISTACHE_USE_SSL

#include <openssl/err.h>
#include <openssl/ssl.h>

#endif /* PISTACHE_USE_SSL */

using namespace std::chrono_literals;

namespace Pistache::Tcp
{

#ifdef PISTACHE_USE_SSL

    namespace
    {

        std::string ssl_print_errors_to_string()
        {
            ssl::SSLBioPtr bio { BIO_new(BIO_s_mem()) };
            ERR_print_errors(GetSSLBio(bio));

            static const int buffer_length = 512;

            bool continue_reading = true;
            char buffer[buffer_length];
            std::string result;

            while (continue_reading)
            {
                int ret = BIO_gets(GetSSLBio(bio), buffer, buffer_length);
                switch (ret)
                {
                case 0:
                case -1:
                    // Reached the end of the BIO, or it is unreadable for some reason.
                    continue_reading = false;
                    break;
                case -2:
                    throw std::logic_error("Trying to call PopStringFromBio on a BIO that "
                                           "does not support the BIO_gets method");
                    break;
                default: // >0
                    result.append(buffer);
                    break;
                }
            }

            return result;
        }

        ssl::SSLCtxPtr ssl_create_context(const std::string& cert,
                                          const std::string& key,
                                          bool use_compression,
                                          int (*cb)(char*, int, int, void*))
        {
            const SSL_METHOD* method = SSLv23_server_method();

            ssl::SSLCtxPtr ctx { SSL_CTX_new(method) };
            if (ctx == nullptr)
            {
                throw std::runtime_error("Cannot setup SSL context");
            }

            if (!use_compression)
            {
                /* Disable compression to prevent BREACH and CRIME vulnerabilities. */
                if (!SSL_CTX_set_options(GetSSLContext(ctx), SSL_OP_NO_COMPRESSION))
                {
                    std::string err = "SSL error - cannot disable compression: "
                        + ssl_print_errors_to_string();
                    throw std::runtime_error(err);
                }
            }

            if (cb != NULL)
            {
                /* Use the user-defined callback for password if provided */
                SSL_CTX_set_default_passwd_cb(GetSSLContext(ctx), cb);
            }

/* Function introduced in 1.0.2 */
#if OPENSSL_VERSION_NUMBER >= 0x10002000L
            SSL_CTX_set_ecdh_auto(GetSSLContext(ctx), 1);
#endif /* OPENSSL_VERSION_NUMBER */

            if (SSL_CTX_use_certificate_chain_file(GetSSLContext(ctx), cert.c_str()) <= 0)
            {
                std::string err = "SSL error - cannot load SSL certificate: "
                    + ssl_print_errors_to_string();
                throw std::runtime_error(err);
            }

            if (SSL_CTX_use_PrivateKey_file(GetSSLContext(ctx), key.c_str(), SSL_FILETYPE_PEM) <= 0)
            {
                std::string err = "SSL error - cannot load SSL private key: "
                    + ssl_print_errors_to_string();
                throw std::runtime_error(err);
            }

            if (!SSL_CTX_check_private_key(GetSSLContext(ctx)))
            {
                std::string err = "SSL error - Private key does not match certificate public key: "
                    + ssl_print_errors_to_string();
                throw std::runtime_error(err);
            }

            SSL_CTX_set_mode(GetSSLContext(ctx), SSL_MODE_ENABLE_PARTIAL_WRITE);
            SSL_CTX_set_mode(GetSSLContext(ctx), SSL_MODE_ACCEPT_MOVING_WRITE_BUFFER);
            return ctx;
        }

    }
#endif /* PISTACHE_USE_SSL */

    void setSocketOptions(Fd fd, Flags<Options> options)
    {
        if (options.hasFlag(Options::ReuseAddr))
        {
            int one = 1;
            TRY(::setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one)));
        }

        if (options.hasFlag(Options::ReusePort))
        {
            int one = 1;
            TRY(::setsockopt(fd, SOL_SOCKET, SO_REUSEPORT, &one, sizeof(one)));
        }

        if (options.hasFlag(Options::Linger))
        {
            struct linger opt;
            opt.l_onoff  = 1;
            opt.l_linger = 1;
            TRY(::setsockopt(fd, SOL_SOCKET, SO_LINGER, &opt, sizeof(opt)));
        }

        if (options.hasFlag(Options::FastOpen))
        {
            int hint = 5;
            TRY(::setsockopt(fd, SOL_TCP, TCP_FASTOPEN, &hint, sizeof(hint)));
        }
        if (options.hasFlag(Options::NoDelay))
        {
            int one = 1;
            TRY(::setsockopt(fd, SOL_TCP, TCP_NODELAY, &one, sizeof(one)));
        }
    }

    Listener::Listener()
        : transportFactory_(defaultTransportFactory())
    { }

    Listener::Listener(const Address& address)
        : addr_(address)
        , transportFactory_(defaultTransportFactory())
    { }

    Listener::~Listener()
    {
        if (isBound())
            shutdown();
        if (acceptThread.joinable())
            acceptThread.join();

        if (listen_fd >= 0)
        {
            close(listen_fd);
            listen_fd = -1;
        }
    }

    void Listener::init(size_t workers, Flags<Options> options,
                        const std::string& workersName, int backlog,
                        PISTACHE_STRING_LOGGER_T logger)
    {
        if (workers > hardware_concurrency())
        {
            // Log::warning() << "More workers than available cores"
        }

        options_     = options;
        backlog_     = backlog;
        useSSL_      = false;
        workers_     = workers;
        workersName_ = workersName;
        logger_      = logger;
    }

    void Listener::setTransportFactory(TransportFactory factory)
    {
        transportFactory_ = std::move(factory);
    }

    void Listener::setHandler(const std::shared_ptr<Handler>& handler)
    {
        handler_ = handler;
    }

    void Listener::pinWorker([[maybe_unused]] size_t worker, [[maybe_unused]] const CpuSet& set)
    {
#if 0
    if (ioGroup.empty()) {
        throw std::domain_error("Invalid operation, did you call init() before ?");
    }
    if (worker > ioGroup.size()) {
        throw std::invalid_argument("Trying to pin invalid worker");
    }

    auto &wrk = ioGroup[worker];
    wrk->pin(set);
#endif
    }

    void Listener::bind() { bind(addr_); }

    void Listener::bind(const Address& address)
    {
        addr_ = address;

        struct addrinfo hints;
        memset(&hints, 0, sizeof(struct addrinfo));
        hints.ai_family   = address.family();
        hints.ai_socktype = SOCK_STREAM;
        hints.ai_flags    = AI_PASSIVE;
        hints.ai_protocol = 0;

        const auto& host = addr_.host();
        const auto& port = addr_.port().toString();
        AddrInfo addr_info;

        TRY(addr_info.invoke(host.c_str(), port.c_str(), &hints));

        int fd = -1;

        const addrinfo* addr = nullptr;
        for (addr = addr_info.get_info_ptr(); addr; addr = addr->ai_next)
        {
            auto socktype = addr->ai_socktype;
            if (options_.hasFlag(Options::CloseOnExec))
                socktype |= SOCK_CLOEXEC;

            fd = ::socket(addr->ai_family, socktype, addr->ai_protocol);
            if (fd < 0)
                continue;

            setSocketOptions(fd, options_);

            if (::bind(fd, addr->ai_addr, addr->ai_addrlen) < 0)
            {
                close(fd);
                continue;
            }

            TRY(::listen(fd, backlog_));
            break;
        }

        // At this point, it is still possible that we couldn't bind any socket. If it
        // is the case, the previous loop would have exited naturally and addr will be
        // null.
        if (addr == nullptr)
        {
            throw std::runtime_error(strerror(errno));
        }

        make_non_blocking(fd);
        poller.addFd(fd, Flags<Polling::NotifyOn>(Polling::NotifyOn::Read),
                     Polling::Tag(fd));
        listen_fd = fd;

        auto transport = transportFactory_();

        reactor_.init(Aio::AsyncContext(workers_, workersName_));
        transportKey = reactor_.addHandler(transport);
    }

    bool Listener::isBound() const { return listen_fd != -1; }

    // Return actual TCP port Listener is on, or 0 on error / no port.
    // Notes:
    // 1) Default constructor for 'Port()' sets value to 0.
    // 2) Socket is created inside 'Listener::run()', which is called from
    //    'Endpoint::serve()' and 'Endpoint::serveThreaded()'.  So getting the
    //    port is only useful if you attempt to do so from a _different_ thread
    //    than the one running 'Listener::run()'.  So for a traditional single-
    //    threaded program this method is of little value.
    Port Listener::getPort() const
    {
        if (listen_fd == -1)
        {
            return Port();
        }

        struct sockaddr_in sock_addr = { 0 };
        socklen_t addrlen            = sizeof(sock_addr);
        auto* sock_addr_alias        = reinterpret_cast<struct sockaddr*>(&sock_addr);

        if (-1 == getsockname(listen_fd, sock_addr_alias, &addrlen))
        {
            return Port();
        }

        return Port(ntohs(sock_addr.sin_port));
    }

    void Listener::run()
    {
        if (!shutdownFd.isBound())
            shutdownFd.bind(poller);
        reactor_.run();

        for (;;)
        {
            std::vector<Polling::Event> events;
            int ready_fds = poller.poll(events);

            if (ready_fds == -1)
            {
                throw Error::system("Polling");
            }
            for (const auto& event : events)
            {
                if (event.tag == shutdownFd.tag())
                    return;

                if (event.flags.hasFlag(Polling::NotifyOn::Read))
                {
                    auto fd = event.tag.value();
                    if (static_cast<ssize_t>(fd) == listen_fd)
                    {
                        try
                        {
                            handleNewConnection();
                        }
                        catch (SocketError& ex)
                        {
                            PISTACHE_LOG_STRING_WARN(logger_, "Socket error: " << ex.what());
                        }
                        catch (ServerError& ex)
                        {
                            PISTACHE_LOG_STRING_FATAL(logger_, "Server error: " << ex.what());
                            throw;
                        }
                    }
                }
            }
        }
    }

    void Listener::runThreaded()
    {
        shutdownFd.bind(poller);
        acceptThread = std::thread([=]() { this->run(); });
    }

    void Listener::shutdown()
    {
        if (shutdownFd.isBound())
            shutdownFd.notify();
        reactor_.shutdown();
    }

    Async::Promise<Listener::Load>
    Listener::requestLoad(const Listener::Load& old)
    {
        auto handlers = reactor_.handlers(transportKey);

        std::vector<Async::Promise<rusage>> loads;
        for (const auto& handler : handlers)
        {
            auto transport = std::static_pointer_cast<Transport>(handler);
            loads.push_back(transport->load());
        }

        return Async::whenAll(std::begin(loads), std::end(loads))
            .then(
                [=](const std::vector<rusage>& usages) {
                    Load res;
                    res.raw = usages;

                    if (old.raw.empty())
                    {
                        res.global = 0.0;
                        for (size_t i = 0; i < handlers.size(); ++i)
                            res.workers.push_back(0.0);
                    }
                    else
                    {

                        auto totalElapsed = [](rusage usage) {
                            return static_cast<double>((usage.ru_stime.tv_sec * 1000000 + usage.ru_stime.tv_usec) + (usage.ru_utime.tv_sec * 1000000 + usage.ru_utime.tv_usec));
                        };

                        auto now  = std::chrono::system_clock::now();
                        auto diff = now - old.tick;
                        auto tick = std::chrono::duration_cast<std::chrono::microseconds>(diff);
                        res.tick  = now;

                        for (size_t i = 0; i < usages.size(); ++i)
                        {
                            auto last         = old.raw[i];
                            const auto& usage = usages[i];

                            auto nowElapsed  = totalElapsed(usage);
                            auto timeElapsed = nowElapsed - totalElapsed(last);

                            auto loadPct = (timeElapsed * 100.0) / static_cast<double>(tick.count());
                            res.workers.push_back(loadPct);
                            res.global += loadPct;
                        }

                        res.global /= static_cast<double>(usages.size());
                    }

                    return res;
                },
                Async::Throw);
    }

    Address Listener::address() const { return addr_; }

    Options Listener::options() const { return options_; }

    void Listener::handleNewConnection()
    {
        struct sockaddr_storage peer_addr;
        int client_fd = acceptConnection(peer_addr);

        void* ssl = nullptr;

#ifdef PISTACHE_USE_SSL
        if (this->useSSL_)
        {

            SSL* ssl_data = SSL_new(GetSSLContext(ssl_ctx_));
            if (ssl_data == nullptr)
            {
                close(client_fd);
                std::string err = "SSL error - cannot create SSL connection: "
                    + ssl_print_errors_to_string();
                throw ServerError(err.c_str());
            }

            // If user requested SSL handshake timeout, enable it on the socket.
            //  This is sometimes necessary if a client connects, sends nothing,
            //  or possibly refuses to accept any bytes, and never completes a
            //  handshake. This would have left SSL_accept hanging indefinitely
            //  and is effectively a DoS...
            if (sslHandshakeTimeout_ > 0ms)
            {
                struct timeval timeout;

                timeout.tv_sec = std::chrono::duration_cast<std::chrono::seconds>(sslHandshakeTimeout_).count();

                const auto residual_microseconds = std::chrono::duration_cast<std::chrono::microseconds>(sslHandshakeTimeout_) - std::chrono::duration_cast<std::chrono::seconds>(sslHandshakeTimeout_);
                timeout.tv_usec                  = residual_microseconds.count();

                TRY(::setsockopt(client_fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout)));
                TRY(::setsockopt(client_fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout)));
            }

            SSL_set_fd(ssl_data, client_fd);
            SSL_set_accept_state(ssl_data);

            if (SSL_accept(ssl_data) <= 0)
            {
                std::string err = "SSL connection error: "
                    + ssl_print_errors_to_string();
                PISTACHE_LOG_STRING_INFO(logger_, err);
                SSL_free(ssl_data);
                close(client_fd);
                return;
            }

            // Remove socket timeouts if they were enabled now that we have
            //  handshaked...
            if (sslHandshakeTimeout_ > 0ms)
            {
                struct timeval timeout;
                timeout.tv_sec  = 0;
                timeout.tv_usec = 0;

                TRY(::setsockopt(client_fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout)));
                TRY(::setsockopt(client_fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout)));
            }

            ssl = static_cast<void*>(ssl_data);
        }
#endif /* PISTACHE_USE_SSL */

        make_non_blocking(client_fd);

        std::shared_ptr<Peer> peer;
        auto* peer_alias = reinterpret_cast<struct sockaddr*>(&peer_addr);
        if (this->useSSL_)
        {
            peer = Peer::CreateSSL(client_fd, Address::fromUnix(peer_alias), ssl);
        }
        else
        {
            peer = Peer::Create(client_fd, Address::fromUnix(peer_alias));
        }

        dispatchPeer(peer);
    }

    int Listener::acceptConnection(struct sockaddr_storage& peer_addr) const
    {
        socklen_t peer_addr_len = sizeof(peer_addr);
        // Do not share open FD with forked processes
        int client_fd = ::accept4(
            listen_fd, reinterpret_cast<struct sockaddr*>(&peer_addr), &peer_addr_len, SOCK_CLOEXEC);
        if (client_fd < 0)
        {
            if (errno == EBADF || errno == ENOTSOCK)
                throw ServerError(strerror(errno));
            else
                throw SocketError(strerror(errno));
        }
        return client_fd;
    }

    void Listener::dispatchPeer(const std::shared_ptr<Peer>& peer)
    {
        auto handlers  = reactor_.handlers(transportKey);
        auto idx       = peer->fd() % handlers.size();
        auto transport = std::static_pointer_cast<Transport>(handlers[idx]);

        transport->handleNewPeer(peer);
    }

    Listener::TransportFactory Listener::defaultTransportFactory() const
    {
        return [&] {
            if (!handler_)
                throw std::runtime_error("setHandler() has not been called");

            return std::make_shared<Transport>(handler_);
        };
    }

#ifdef PISTACHE_USE_SSL

    void Listener::setupSSLAuth(const std::string& ca_file,
                                const std::string& ca_path,
                                int (*cb)(int, void*) = NULL)
    {
        const char* __ca_file = NULL;
        const char* __ca_path = NULL;

        if (ssl_ctx_ == nullptr)
        {
            std::string err = "SSL Context is not initialized";
            PISTACHE_LOG_STRING_FATAL(logger_, err);
            throw std::runtime_error(err);
        }

        if (!ca_file.empty())
            __ca_file = ca_file.c_str();
        if (!ca_path.empty())
            __ca_path = ca_path.c_str();

        if (SSL_CTX_load_verify_locations(GetSSLContext(ssl_ctx_), __ca_file,
                                          __ca_path)
            <= 0)
        {
            std::string err = "SSL error - Cannot verify SSL locations: "
                + ssl_print_errors_to_string();
            PISTACHE_LOG_STRING_FATAL(logger_, err);
            throw std::runtime_error(err);
        }

        SSL_CTX_set_verify(GetSSLContext(ssl_ctx_),
                           SSL_VERIFY_PEER | SSL_VERIFY_FAIL_IF_NO_PEER_CERT | SSL_VERIFY_CLIENT_ONCE,
/* Callback type did change in 1.0.1 */
#if OPENSSL_VERSION_NUMBER < 0x10100000L || defined(LIBRESSL_VERSION_NUMBER)
                           (int (*)(int, X509_STORE_CTX*))cb
#else
                           (SSL_verify_cb)cb
#endif /* OPENSSL_VERSION_NUMBER */
        );
    }

    void Listener::setupSSL(const std::string& cert_path,
                            const std::string& key_path,
                            bool use_compression,
                            int (*cb_password)(char*, int, int, void*),
                            std::chrono::milliseconds sslHandshakeTimeout)
    {
        SSL_load_error_strings();
        OpenSSL_add_ssl_algorithms();

        try
        {
            ssl_ctx_ = ssl_create_context(cert_path, key_path, use_compression, cb_password);
        }
        catch (std::exception& e)
        {
            PISTACHE_LOG_STRING_FATAL(logger_, e.what());
            throw;
        }
        sslHandshakeTimeout_ = sslHandshakeTimeout;
        useSSL_              = true;
    }

#endif /* PISTACHE_USE_SSL */

    std::vector<std::shared_ptr<Tcp::Peer>> Listener::getAllPeer()
    {
        std::vector<std::shared_ptr<Tcp::Peer>> vecPeers;
        auto handlers = reactor_.handlers(transportKey);

        for (const auto& handler : handlers)
        {
            auto transport = std::static_pointer_cast<Transport>(handler);
            auto peers     = transport->getAllPeer();
            vecPeers.insert(vecPeers.end(), peers.begin(), peers.end());
        }
        return vecPeers;
    }

} // namespace Pistache::Tcp
