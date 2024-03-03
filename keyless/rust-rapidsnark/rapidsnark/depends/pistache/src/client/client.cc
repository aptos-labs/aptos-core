/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 29 janvier 2016

   Implementation of the Http client
*/

#include <pistache/client.h>
#include <pistache/common.h>
#include <pistache/http.h>
#include <pistache/net.h>
#include <pistache/stream.h>

#include <netdb.h>
#include <sys/sendfile.h>
#include <sys/socket.h>
#include <sys/types.h>

#include <algorithm>
#include <memory>
#include <sstream>
#include <string>

namespace Pistache::Http::Experimental
{
    using NotifyOn = Polling::NotifyOn;

    static constexpr const char* UA = "pistache/0.1";

    namespace
    {
        // Using const_cast can result in undefined behavior.
        // C++17 provides a non-const .data() overload,
        // but url must be passed as a non-const reference (or by value)
        std::pair<std::string_view, std::string_view> splitUrl(const std::string& url)
        {
            RawStreamBuf<char> buf(const_cast<char*>(url.data()), url.size());
            StreamCursor cursor(&buf);

            match_string("http://", cursor);
            match_string("www", cursor);
            match_literal('.', cursor);

            StreamCursor::Token hostToken(cursor);
            match_until({ '?', '/' }, cursor);

            std::string_view host(hostToken.rawText(), hostToken.size());
            std::string_view page(cursor.offset(), buf.endptr() - buf.curptr());

            return std::make_pair(host, page);
        }
    } // namespace

    namespace
    {
        template <typename H, typename... Args>
        void writeHeader(std::stringstream& streamBuf, Args&&... args)
        {
            using Http::crlf;

            H header(std::forward<Args>(args)...);

            streamBuf << H::Name << ": ";
            header.write(streamBuf);

            streamBuf << crlf;
        }

        void writeHeaders(std::stringstream& streamBuf,
                          const Http::Header::Collection& headers)
        {
            using Http::crlf;

            for (const auto& header : headers.list())
            {
                streamBuf << header->name() << ": ";
                header->write(streamBuf);
                streamBuf << crlf;
            }
        }

        void writeCookies(std::stringstream& streamBuf,
                          const Http::CookieJar& cookies)
        {
            using Http::crlf;

            streamBuf << "Cookie: ";
            bool first = true;
            for (const auto& cookie : cookies)
            {
                if (!first)
                {
                    streamBuf << "; ";
                }
                else
                {
                    first = false;
                }
                streamBuf << cookie.name << "=" << cookie.value;
            }

            streamBuf << crlf;
        }

        void writeRequest(std::stringstream& streamBuf, const Http::Request& request)
        {
            using Http::crlf;

            const auto& res         = request.resource();
            const auto [host, path] = splitUrl(res);
            const auto& body        = request.body();
            const auto& query       = request.query();

            auto pathStr = std::string(path);

            streamBuf << request.method() << " ";
            if (pathStr[0] != '/')
                streamBuf << '/';

            streamBuf << pathStr << query.as_str();
            streamBuf << " HTTP/1.1" << crlf;

            writeCookies(streamBuf, request.cookies());
            writeHeaders(streamBuf, request.headers());

            writeHeader<Http::Header::UserAgent>(streamBuf, UA);
            writeHeader<Http::Header::Host>(streamBuf, std::string(host));
            if (!body.empty())
            {
                writeHeader<Http::Header::ContentLength>(streamBuf, body.size());
            }
            streamBuf << crlf;

            if (!body.empty())
            {
                streamBuf << body;
            }
        }
    } // namespace

    class Transport : public Aio::Handler
    {
    public:
        PROTOTYPE_OF(Aio::Handler, Transport)

        Transport() = default;
        Transport(const Transport&)
            : requestsQueue()
            , connectionsQueue()
            , connections()
            , timeouts()
            , timeoutsLock()
        { }

        void onReady(const Aio::FdSet& fds) override;
        void registerPoller(Polling::Epoll& poller) override;

        Async::Promise<void> asyncConnect(std::shared_ptr<Connection> connection,
                                          const struct sockaddr* address,
                                          socklen_t addr_len);

        Async::Promise<ssize_t>
        asyncSendRequest(std::shared_ptr<Connection> connection,
                         std::shared_ptr<TimerPool::Entry> timer, std::string buffer);

    private:
        enum WriteStatus { FirstTry,
                           Retry };

        struct ConnectionEntry
        {
            ConnectionEntry(Async::Resolver resolve, Async::Rejection reject,
                            std::shared_ptr<Connection> connection,
                            const struct sockaddr* _addr, socklen_t _addr_len)
                : resolve(std::move(resolve))
                , reject(std::move(reject))
                , connection(connection)
                , addr_len(_addr_len)
            {
                memcpy(&addr, _addr, addr_len);
            }

            const sockaddr* getAddr() const
            {
                return reinterpret_cast<const sockaddr*>(&addr);
            }

            Async::Resolver resolve;
            Async::Rejection reject;
            std::weak_ptr<Connection> connection;
            sockaddr_storage addr;
            socklen_t addr_len;
        };

        struct RequestEntry
        {
            RequestEntry(Async::Resolver resolve, Async::Rejection reject,
                         std::shared_ptr<Connection> connection,
                         std::shared_ptr<TimerPool::Entry> timer, std::string buf)
                : resolve(std::move(resolve))
                , reject(std::move(reject))
                , connection(connection)
                , timer(timer)
                , buffer(std::move(buf))
            { }

            Async::Resolver resolve;
            Async::Rejection reject;
            std::weak_ptr<Connection> connection;
            std::shared_ptr<TimerPool::Entry> timer;
            std::string buffer;
        };

        PollableQueue<RequestEntry> requestsQueue;
        PollableQueue<ConnectionEntry> connectionsQueue;

        std::unordered_map<Fd, ConnectionEntry> connections;
        std::unordered_map<Fd, std::weak_ptr<Connection>> timeouts;

        using Lock  = std::mutex;
        using Guard = std::lock_guard<Lock>;
        Lock timeoutsLock;

    private:
        void asyncSendRequestImpl(const RequestEntry& req,
                                  WriteStatus status = FirstTry);

        void handleRequestsQueue();
        void handleConnectionQueue();
        void handleReadableEntry(const Aio::FdSet::Entry& entry);
        void handleWritableEntry(const Aio::FdSet::Entry& entry);
        void handleHangupEntry(const Aio::FdSet::Entry& entry);
        void handleIncoming(std::shared_ptr<Connection> connection);
    };

    void Transport::onReady(const Aio::FdSet& fds)
    {
        for (const auto& entry : fds)
        {
            if (entry.getTag() == connectionsQueue.tag())
            {
                handleConnectionQueue();
            }
            else if (entry.getTag() == requestsQueue.tag())
            {
                handleRequestsQueue();
            }
            else if (entry.isReadable())
            {
                handleReadableEntry(entry);
            }
            else if (entry.isWritable())
            {
                handleWritableEntry(entry);
            }
            else if (entry.isHangup())
            {
                handleHangupEntry(entry);
            }
            else
            {
                assert(false && "Unexpected event in entry");
            }
        }
    }

    void Transport::registerPoller(Polling::Epoll& poller)
    {
        requestsQueue.bind(poller);
        connectionsQueue.bind(poller);
    }

    Async::Promise<void>
    Transport::asyncConnect(std::shared_ptr<Connection> connection,
                            const struct sockaddr* address, socklen_t addr_len)
    {
        return Async::Promise<void>(
            [=](Async::Resolver& resolve, Async::Rejection& reject) {
                ConnectionEntry entry(std::move(resolve), std::move(reject), connection,
                                      address, addr_len);
                connectionsQueue.push(std::move(entry));
            });
    }

    Async::Promise<ssize_t>
    Transport::asyncSendRequest(std::shared_ptr<Connection> connection,
                                std::shared_ptr<TimerPool::Entry> timer,
                                std::string buffer)
    {

        return Async::Promise<ssize_t>(
            [&](Async::Resolver& resolve, Async::Rejection& reject) {
                auto ctx = context();
                RequestEntry req(std::move(resolve), std::move(reject), connection,
                                 timer, std::move(buffer));
                if (std::this_thread::get_id() != ctx.thread())
                {
                    requestsQueue.push(std::move(req));
                }
                else
                {
                    asyncSendRequestImpl(req);
                }
            });
    }

    void Transport::asyncSendRequestImpl(const RequestEntry& req,
                                         WriteStatus status)
    {
        const auto& buffer = req.buffer;
        auto conn          = req.connection.lock();
        if (!conn)
            throw std::runtime_error("Send request error");

        auto fd = conn->fd();

        ssize_t totalWritten = 0;
        for (;;)
        {
            const char* data           = buffer.data() + totalWritten;
            const ssize_t len          = buffer.size() - totalWritten;
            const ssize_t bytesWritten = ::send(fd, data, len, 0);
            if (bytesWritten < 0)
            {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                {
                    if (status == FirstTry)
                    {
                        throw std::runtime_error("Unimplemented, fix me!");
                    }
                    reactor()->modifyFd(key(), fd, NotifyOn::Write, Polling::Mode::Edge);
                }
                else
                {
                    conn->handleError("Could not send request");
                }
                break;
            }
            else
            {
                totalWritten += bytesWritten;
                if (totalWritten == len)
                {
                    if (req.timer)
                    {
                        Guard guard(timeoutsLock);
                        timeouts.insert(std::make_pair(req.timer->fd(), conn));
                        req.timer->registerReactor(key(), reactor());
                    }
                    req.resolve(totalWritten);
                    break;
                }
            }
        }
    }

    void Transport::handleRequestsQueue()
    {
        // Let's drain the queue
        for (;;)
        {
            auto req = requestsQueue.popSafe();
            if (!req)
                break;

            asyncSendRequestImpl(*req);
        }
    }

    void Transport::handleConnectionQueue()
    {
        for (;;)
        {
            auto data = connectionsQueue.popSafe();
            if (!data)
                break;

            auto conn = data->connection.lock();
            if (!conn)
            {
                data->reject(Error::system("Failed to connect"));
                continue;
            }

            int res = ::connect(conn->fd(), data->getAddr(), data->addr_len);
            if (res == -1)
            {
                if (errno == EINPROGRESS)
                {
                    reactor()->registerFdOneShot(key(), conn->fd(),
                                                 NotifyOn::Write | NotifyOn::Hangup | NotifyOn::Shutdown);
                }
                else
                {
                    data->reject(Error::system("Failed to connect"));
                    continue;
                }
            }
            connections.insert(std::make_pair(conn->fd(), std::move(*data)));
        }
    }

    void Transport::handleReadableEntry(const Aio::FdSet::Entry& entry)
    {
        assert(entry.isReadable() && "Entry must be readable");

        auto tag      = entry.getTag();
        const auto fd = static_cast<Fd>(tag.value());
        auto connIt   = connections.find(fd);
        if (connIt != std::end(connections))
        {
            auto connection = connIt->second.connection.lock();
            if (connection)
            {
                handleIncoming(connection);
            }
            else
            {
                throw std::runtime_error(
                    "Connection error: problem with reading data from server");
            }
        }
        else
        {
            Guard guard(timeoutsLock);
            auto timerIt = timeouts.find(fd);
            if (timerIt != std::end(timeouts))
            {
                auto connection = timerIt->second.lock();
                if (connection)
                {
                    connection->handleTimeout();
                    timeouts.erase(fd);
                }
            }
        }
    }

    void Transport::handleWritableEntry(const Aio::FdSet::Entry& entry)
    {
        assert(entry.isWritable() && "Entry must be writable");

        auto tag      = entry.getTag();
        const auto fd = static_cast<Fd>(tag.value());
        auto connIt   = connections.find(fd);
        if (connIt != std::end(connections))
        {
            auto& connectionEntry = connIt->second;
            auto connection       = connIt->second.connection.lock();
            if (connection)
            {
                connectionEntry.resolve();
                // We are connected, we can start reading data now
                reactor()->modifyFd(key(), connection->fd(), NotifyOn::Read);
            }
            else
            {
                connectionEntry.reject(Error::system("Connection lost"));
            }
        }
        else
        {
            throw std::runtime_error("Unknown fd");
        }
    }

    void Transport::handleHangupEntry(const Aio::FdSet::Entry& entry)
    {
        assert(entry.isHangup() && "Entry must be hangup");

        auto tag      = entry.getTag();
        const auto fd = static_cast<Fd>(tag.value());
        auto connIt   = connections.find(fd);
        if (connIt != std::end(connections))
        {
            auto& connectionEntry = connIt->second;
            connectionEntry.reject(Error::system("Could not connect"));
        }
        else
        {
            throw std::runtime_error("Unknown fd");
        }
    }

    void Transport::handleIncoming(std::shared_ptr<Connection> connection)
    {
        ssize_t totalBytes = 0;

        for (;;)
        {
            char buffer[Const::MaxBuffer] = {
                0,
            };
            const ssize_t bytes = recv(connection->fd(), buffer, Const::MaxBuffer, 0);
            if (bytes == -1)
            {
                if (errno != EAGAIN && errno != EWOULDBLOCK)
                {
                    connection->handleError(strerror(errno));
                }
                break;
            }
            else if (bytes == 0)
            {
                if (totalBytes == 0)
                {
                    connection->handleError("Remote closed connection");
                }
                connections.erase(connection->fd());
                connection->close();
                break;
            }
            else
            {
                totalBytes += bytes;
                connection->handleResponsePacket(buffer, bytes);
            }
        }
    }

    Connection::Connection(size_t maxResponseSize)
        : fd_(-1)
        , requestEntry(nullptr)
        , parser(maxResponseSize)
    {
        state_.store(static_cast<uint32_t>(State::Idle));
        connectionState_.store(NotConnected);
    }

    void Connection::connect(const Address& addr)
    {
        struct addrinfo hints;
        memset(&hints, 0, sizeof(struct addrinfo));
        hints.ai_family   = addr.family();
        hints.ai_socktype = SOCK_STREAM; /* Stream socket */
        hints.ai_flags    = 0;
        hints.ai_protocol = 0;

        const auto& host = addr.host();
        const auto& port = addr.port().toString();

        AddrInfo addressInfo;

        TRY(addressInfo.invoke(host.c_str(), port.c_str(), &hints));
        const addrinfo* addrs = addressInfo.get_info_ptr();

        int sfd = -1;

        for (const addrinfo* addr = addrs; addr; addr = addr->ai_next)
        {
            sfd = ::socket(addr->ai_family, addr->ai_socktype, addr->ai_protocol);
            if (sfd < 0)
                continue;

            make_non_blocking(sfd);

            connectionState_.store(Connecting);
            fd_ = sfd;

            transport_
                ->asyncConnect(shared_from_this(), addr->ai_addr, addr->ai_addrlen)
                .then(
                    [=]() {
                        socklen_t len = sizeof(saddr);
                        getsockname(sfd, reinterpret_cast<struct sockaddr*>(&saddr), &len);
                        connectionState_.store(Connected);
                        processRequestQueue();
                    },
                    PrintException());
            break;
        }

        if (sfd < 0)
            throw std::runtime_error("Failed to connect");
    }

    std::string Connection::dump() const
    {
        std::ostringstream oss;
        oss << "Connection(fd = " << fd_ << ", src_port = ";
        oss << ntohs(saddr.sin_port) << ")";
        return oss.str();
    }

    bool Connection::isIdle() const
    {
        return static_cast<Connection::State>(state_.load()) == Connection::State::Idle;
    }

    bool Connection::tryUse()
    {
        auto curState = static_cast<uint32_t>(Connection::State::Idle);
        auto newState = static_cast<uint32_t>(Connection::State::Used);
        return state_.compare_exchange_strong(curState, newState);
    }

    void Connection::setAsIdle()
    {
        state_.store(static_cast<uint32_t>(Connection::State::Idle));
    }

    bool Connection::isConnected() const
    {
        return connectionState_.load() == Connected;
    }

    void Connection::close()
    {
        connectionState_.store(NotConnected);
        ::close(fd_);
    }

    void Connection::associateTransport(
        const std::shared_ptr<Transport>& transport)
    {
        if (transport_)
            throw std::runtime_error(
                "A transport has already been associated to the connection");

        transport_ = transport;
    }

    bool Connection::hasTransport() const { return transport_ != nullptr; }

    Fd Connection::fd() const
    {
        assert(fd_ != -1);
        return fd_;
    }

    void Connection::handleResponsePacket(const char* buffer, size_t totalBytes)
    {
        try
        {
            const bool result = parser.feed(buffer, totalBytes);
            if (!result)
            {
                handleError("Client: Too long packet");
                return;
            }
            if (parser.parse() == Private::State::Done)
            {
                if (requestEntry)
                {
                    if (requestEntry->timer)
                    {
                        requestEntry->timer->disarm();
                        timerPool_.releaseTimer(requestEntry->timer);
                    }

                    requestEntry->resolve(std::move(parser.response));
                    parser.reset();

                    auto onDone = requestEntry->onDone;

                    requestEntry.reset(nullptr);

                    if (onDone)
                        onDone();
                }
            }
        }
        catch (const std::exception& ex)
        {
            handleError(ex.what());
        }
    }

    void Connection::handleError(const char* error)
    {
        if (requestEntry)
        {
            if (requestEntry->timer)
            {
                requestEntry->timer->disarm();
                timerPool_.releaseTimer(requestEntry->timer);
            }

            auto onDone = requestEntry->onDone;

            requestEntry->reject(Error(error));

            requestEntry.reset(nullptr);

            if (onDone)
                onDone();
        }
    }

    void Connection::handleTimeout()
    {
        if (requestEntry)
        {
            requestEntry->timer->disarm();
            timerPool_.releaseTimer(requestEntry->timer);

            auto onDone = requestEntry->onDone;

            /* @API: create a TimeoutException */
            requestEntry->reject(std::runtime_error("Timeout"));

            requestEntry.reset(nullptr);

            if (onDone)
                onDone();
        }
    }

    Async::Promise<Response> Connection::perform(const Http::Request& request,
                                                 Connection::OnDone onDone)
    {
        return Async::Promise<Response>(
            [=](Async::Resolver& resolve, Async::Rejection& reject) {
                performImpl(request, std::move(resolve), std::move(reject),
                            std::move(onDone));
            });
    }

    Async::Promise<Response> Connection::asyncPerform(const Http::Request& request,
                                                      Connection::OnDone onDone)
    {
        return Async::Promise<Response>(
            [=](Async::Resolver& resolve, Async::Rejection& reject) {
                requestsQueue.push(RequestData(std::move(resolve), std::move(reject),
                                               request, std::move(onDone)));
            });
    }

    void Connection::performImpl(const Http::Request& request,
                                 Async::Resolver resolve, Async::Rejection reject,
                                 Connection::OnDone onDone)
    {

        std::stringstream streamBuf;
        writeRequest(streamBuf, request);
        if (!streamBuf)
            reject(std::runtime_error("Could not write request"));
        std::string buffer = streamBuf.str();

        std::shared_ptr<TimerPool::Entry> timer(nullptr);
        auto timeout = request.timeout();
        if (timeout.count() > 0)
        {
            timer = timerPool_.pickTimer();
            timer->arm(timeout);
        }

        requestEntry = std::make_unique<RequestEntry>(std::move(resolve), std::move(reject),
                                                      timer, std::move(onDone));
        transport_->asyncSendRequest(shared_from_this(), timer, std::move(buffer));
    }

    void Connection::processRequestQueue()
    {
        for (;;)
        {
            auto req = requestsQueue.popSafe();
            if (!req)
                break;

            performImpl(req->request, std::move(req->resolve), std::move(req->reject),
                        std::move(req->onDone));
        }
    }

    void ConnectionPool::init(size_t maxConnectionsPerHost,
                              size_t maxResponseSize)
    {
        this->maxConnectionsPerHost = maxConnectionsPerHost;
        this->maxResponseSize       = maxResponseSize;
    }

    std::shared_ptr<Connection>
    ConnectionPool::pickConnection(const std::string& domain)
    {
        Connections pool;

        {
            Guard guard(connsLock);
            auto poolIt = conns.find(domain);
            if (poolIt == std::end(conns))
            {
                Connections connections;
                for (size_t i = 0; i < maxConnectionsPerHost; ++i)
                {
                    connections.push_back(std::make_shared<Connection>(maxResponseSize));
                }

                poolIt = conns.insert(std::make_pair(domain, std::move(connections))).first;
            }
            pool = poolIt->second;
        }

        for (auto& conn : pool)
        {
            if (conn->tryUse())
            {
                return conn;
            }
        }

        return nullptr;
    }

    void ConnectionPool::releaseConnection(
        const std::shared_ptr<Connection>& connection)
    {
        connection->setAsIdle();
    }

    size_t ConnectionPool::usedConnections(const std::string& domain) const
    {
        Connections pool;
        {
            Guard guard(connsLock);
            auto it = conns.find(domain);
            if (it == std::end(conns))
            {
                return 0;
            }
            pool = it->second;
        }

        return std::count_if(pool.begin(), pool.end(),
                             [](const std::shared_ptr<Connection>& conn) {
                                 return conn->isConnected();
                             });
    }

    size_t ConnectionPool::idleConnections(const std::string& domain) const
    {
        Connections pool;
        {
            Guard guard(connsLock);
            auto it = conns.find(domain);
            if (it == std::end(conns))
            {
                return 0;
            }
            pool = it->second;
        }

        return std::count_if(
            pool.begin(), pool.end(),
            [](const std::shared_ptr<Connection>& conn) { return conn->isIdle(); });
    }

    size_t ConnectionPool::availableConnections(const std::string& /*domain*/) const
    {
        return 0;
    }

    void ConnectionPool::closeIdleConnections(const std::string& /*domain*/)
    {
    }

    void ConnectionPool::shutdown()
    {
        // close all connections
        Guard guard(connsLock);
        for (auto& it : conns)
        {
            for (auto& conn : it.second)
            {
                if (conn->isConnected())
                {
                    conn->close();
                }
            }
        }
    }

    RequestBuilder& RequestBuilder::method(Method method)
    {
        request_.method_ = method;
        return *this;
    }

    RequestBuilder& RequestBuilder::resource(const std::string& val)
    {
        request_.resource_ = val;
        return *this;
    }

    RequestBuilder& RequestBuilder::params(const Uri::Query& query)
    {
        request_.query_ = query;
        return *this;
    }

    RequestBuilder&
    RequestBuilder::header(const std::shared_ptr<Header::Header>& header)
    {
        request_.headers_.add(header);
        return *this;
    }

    RequestBuilder& RequestBuilder::cookie(const Cookie& cookie)
    {
        request_.cookies_.add(cookie);
        return *this;
    }

    RequestBuilder& RequestBuilder::body(const std::string& val)
    {
        request_.body_ = val;
        return *this;
    }

    RequestBuilder& RequestBuilder::body(std::string&& val)
    {
        request_.body_ = std::move(val);
        return *this;
    }

    RequestBuilder& RequestBuilder::timeout(std::chrono::milliseconds val)
    {
        request_.timeout_ = val;
        return *this;
    }

    Async::Promise<Response> RequestBuilder::send()
    {
        return client_->doRequest(request_);
    }

    Client::Options& Client::Options::threads(int val)
    {
        threads_ = val;
        return *this;
    }

    Client::Options& Client::Options::keepAlive(bool val)
    {
        keepAlive_ = val;
        return *this;
    }

    Client::Options& Client::Options::maxConnectionsPerHost(int val)
    {
        maxConnectionsPerHost_ = val;
        return *this;
    }

    Client::Options& Client::Options::maxResponseSize(size_t val)
    {
        maxResponseSize_ = val;
        return *this;
    }

    Client::Client()
        : reactor_(Aio::Reactor::create())
        , pool()
        , transportKey()
        , ioIndex(0)
        , queuesLock()
        , requestsQueues()
        , stopProcessPequestsQueues(false)
    { }

    Client::~Client()
    {
        assert(stopProcessPequestsQueues == true && "You must explicitly call shutdown method of Client object");
    }

    Client::Options Client::options() { return Client::Options(); }

    void Client::init(const Client::Options& options)
    {
        pool.init(options.maxConnectionsPerHost_, options.maxResponseSize_);
        reactor_->init(Aio::AsyncContext(options.threads_));
        transportKey = reactor_->addHandler(std::make_shared<Transport>());
        reactor_->run();
    }

    void Client::shutdown()
    {
        reactor_->shutdown();
        pool.shutdown();
        Guard guard(queuesLock);
        stopProcessPequestsQueues = true;
    }

    RequestBuilder Client::get(const std::string& resource)
    {
        return prepareRequest(resource, Http::Method::Get);
    }

    RequestBuilder Client::post(const std::string& resource)
    {
        return prepareRequest(resource, Http::Method::Post);
    }

    RequestBuilder Client::put(const std::string& resource)
    {
        return prepareRequest(resource, Http::Method::Put);
    }

    RequestBuilder Client::patch(const std::string& resource)
    {
        return prepareRequest(resource, Http::Method::Patch);
    }

    RequestBuilder Client::del(const std::string& resource)
    {
        return prepareRequest(resource, Http::Method::Delete);
    }

    RequestBuilder Client::prepareRequest(const std::string& resource,
                                          Http::Method method)
    {
        RequestBuilder builder(this);
        builder.resource(resource).method(method);

        return builder;
    }

    Async::Promise<Response> Client::doRequest(Http::Request request)
    {
        // request.headers_.add<Header::Connection>(ConnectionControl::KeepAlive);
        request.headers().remove<Header::UserAgent>();
        auto resourceData = request.resource();

        auto resource = splitUrl(resourceData);
        auto conn     = pool.pickConnection(std::string(resource.first));

        if (conn == nullptr)
        {
            return Async::Promise<Response>([this, resource = std::move(resource),
                                             request](Async::Resolver& resolve,
                                                      Async::Rejection& reject) {
                Guard guard(queuesLock);

                auto data = std::make_shared<Connection::RequestData>(
                    std::move(resolve), std::move(reject), std::move(request), nullptr);
                auto& queue = requestsQueues[std::string(resource.first)];
                if (!queue.enqueue(data))
                    data->reject(std::runtime_error("Queue is full"));
            });
        }
        else
        {

            if (!conn->hasTransport())
            {
                auto transports = reactor_->handlers(transportKey);
                auto index      = ioIndex.fetch_add(1) % transports.size();

                auto transport = std::static_pointer_cast<Transport>(transports[index]);
                conn->associateTransport(transport);
            }

            if (!conn->isConnected())
            {
                std::weak_ptr<Connection> weakConn = conn;
                auto res                           = conn->asyncPerform(request, [this, weakConn]() {
                    auto conn = weakConn.lock();
                    if (conn)
                    {
                        pool.releaseConnection(conn);
                        processRequestQueue();
                    }
                });
                conn->connect(helpers::httpAddr(resource.first));
                return res;
            }

            std::weak_ptr<Connection> weakConn = conn;
            return conn->perform(request, [this, weakConn]() {
                auto conn = weakConn.lock();
                if (conn)
                {
                    pool.releaseConnection(conn);
                    processRequestQueue();
                }
            });
        }
    }

    void Client::processRequestQueue()
    {
        Guard guard(queuesLock);

        if (stopProcessPequestsQueues)
            return;

        for (auto& queues : requestsQueues)
        {
            for (;;)
            {
                const auto& domain = queues.first;
                auto conn          = pool.pickConnection(domain);
                if (!conn)
                    break;

                auto& queue = queues.second;
                std::shared_ptr<Connection::RequestData> data;
                if (!queue.dequeue(data))
                {
                    pool.releaseConnection(conn);
                    break;
                }

                conn->performImpl(data->request, std::move(data->resolve),
                                  std::move(data->reject), [this, conn]() {
                                      pool.releaseConnection(conn);
                                      processRequestQueue();
                                  });
            }
        }
    }

} // namespace Pistache::Http
