/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* router.cc
   Mathieu Stefani, 05 janvier 2016

   Rest routing implementation
*/

#include <algorithm>

#include <pistache/description.h>
#include <pistache/router.h>

namespace Pistache::Rest
{

    Request::Request(Http::Request request, std::vector<TypedParam>&& params,
                     std::vector<TypedParam>&& splats)
        : Http::Request(std::move(request))
        , params_(std::move(params))
        , splats_(std::move(splats))
    { }

    bool Request::hasParam(const std::string& name) const
    {
        auto it = std::find_if(
            params_.begin(), params_.end(),
            [&](const TypedParam& param) { return param.name() == name; });

        return it != std::end(params_);
    }

    TypedParam Request::param(const std::string& name) const
    {
        auto it = std::find_if(
            params_.begin(), params_.end(),
            [&](const TypedParam& param) { return param.name() == name; });

        if (it == std::end(params_))
        {
            throw std::runtime_error("Unknown parameter");
        }

        return *it;
    }

    TypedParam Request::splatAt(size_t index) const
    {
        if (index >= splats_.size())
        {
            throw std::out_of_range("Request splat index out of range");
        }
        return splats_[index];
    }

    std::vector<TypedParam> Request::splat() const { return splats_; }

    std::regex SegmentTreeNode::multiple_slash = std::regex("//+", std::regex_constants::optimize);

    SegmentTreeNode::SegmentTreeNode()
        : resource_ref_()
        , fixed_()
        , param_()
        , optional_()
        , splat_(nullptr)
        , route_(nullptr)
    {
        std::shared_ptr<char> ptr(new char[0], std::default_delete<char[]>());
        resource_ref_.swap(ptr);
    }

    SegmentTreeNode::SegmentTreeNode(const std::shared_ptr<char>& resourceReference)
        : resource_ref_(resourceReference)
        , fixed_()
        , param_()
        , optional_()
        , splat_(nullptr)
        , route_(nullptr)
    { }

    SegmentTreeNode::SegmentType
    SegmentTreeNode::getSegmentType(const std::string_view& fragment)
    {
        auto optpos = fragment.find('?');
        if (fragment[0] == ':')
        {
            if (optpos != std::string_view::npos)
            {
                if (optpos != fragment.length() - 1)
                {
                    throw std::runtime_error("? should be at the end of the string");
                }
                return SegmentType::Optional;
            }
            return SegmentType::Param;
        }
        else if (fragment[0] == '*')
        {
            if (fragment.length() > 1)
            {
                throw std::runtime_error("Invalid splat parameter");
            }
            return SegmentType::Splat;
        }

        if (optpos != std::string_view::npos)
        {
            throw std::runtime_error(
                "Only optional parameters are currently supported");
        }

        return SegmentType::Fixed;
    }

    std::string SegmentTreeNode::sanitizeResource(const std::string& path)
    {
        const auto& dup = std::regex_replace(path, SegmentTreeNode::multiple_slash,
                                             std::string("/"));
        if (dup[dup.length() - 1] == '/')
        {
            return dup.substr(1, dup.length() - 2);
        }
        return dup.substr(1);
    }

    void SegmentTreeNode::addRoute(
        const std::string_view& path, const Route::Handler& handler,
        const std::shared_ptr<char>& resource_reference)
    {
        // recursion to correct path segment
        if (!path.empty())
        {
            const auto segment_delimiter = path.find('/');
            // current segment value
            auto current_segment = path.substr(0, segment_delimiter);
            // complete child path (path without this segment)
            // if no '/' was found, it means that it is a leaf resource
            const auto lower_path = (segment_delimiter == std::string_view::npos)
                ? std::string_view { nullptr, 0 }
                : path.substr(segment_delimiter + 1);

            std::unordered_map<std::string_view, std::shared_ptr<SegmentTreeNode>>* collection = nullptr;
            const auto fragmentType                                                            = getSegmentType(current_segment);
            switch (fragmentType)
            {
            case SegmentType::Fixed:
                collection = &fixed_;
                break;
            case SegmentType::Param:
                collection = &param_;
                break;
            case SegmentType::Optional:
                // remove the trailing question mark
                current_segment = current_segment.substr(0, current_segment.length() - 1);
                collection      = &optional_;
                break;
            case SegmentType::Splat:
                if (splat_ == nullptr)
                {
                    splat_ = std::make_shared<SegmentTreeNode>(resource_reference);
                }
                splat_->addRoute(lower_path, handler, resource_reference);
                return;
            }

            // if the segment tree nodes for the lower path does not exist
            if (collection->count(current_segment) == 0)
            {
                // first create it
                collection->insert(std::make_pair(
                    current_segment,
                    std::make_shared<SegmentTreeNode>(resource_reference)));
            }
            collection->at(current_segment)
                ->addRoute(lower_path, handler, resource_reference);
        }
        else
        { // current path segment requested
            if (route_ != nullptr)
                throw std::runtime_error("Requested route already exist.");
            route_ = std::make_shared<Route>(handler);
        }
    }

    bool Pistache::Rest::SegmentTreeNode::removeRoute(
        const std::string_view& path)
    {
        // recursion to correct path segment
        if (!path.empty())
        {
            const auto segment_delimiter = path.find('/');
            // current segment value
            auto current_segment = path.substr(0, segment_delimiter);
            // complete child path (path without this segment)
            // if no '/' was found, it means that it is a leaf resource
            const auto lower_path = (segment_delimiter == std::string_view::npos)
                ? std::string_view { nullptr, 0 }
                : path.substr(segment_delimiter + 1);

            std::unordered_map<std::string_view, std::shared_ptr<SegmentTreeNode>>* collection = nullptr;
            auto fragmentType                                                                  = getSegmentType(current_segment);
            switch (fragmentType)
            {
            case SegmentType::Fixed:
                collection = &fixed_;
                break;
            case SegmentType::Param:
                collection = &param_;
                break;
            case SegmentType::Optional:
                // remove the trailing question mark
                current_segment = current_segment.substr(0, current_segment.length() - 1);
                collection      = &optional_;
                break;
            case SegmentType::Splat:
                return splat_->removeRoute(lower_path);
            }

            try
            {
                const bool removable = collection->at(current_segment)->removeRoute(lower_path);
                if (removable)
                {
                    collection->erase(current_segment);
                }
            }
            catch (const std::out_of_range&)
            {
                throw std::runtime_error("Requested does not exist.");
            }
        }
        else
        { // current leaf requested
            route_.reset();
        }
        return fixed_.empty() && param_.empty() && optional_.empty() && splat_ == nullptr && route_ == nullptr;
    }

    std::tuple<std::shared_ptr<Route>, std::vector<TypedParam>,
               std::vector<TypedParam>>
    Pistache::Rest::SegmentTreeNode::findRoute(
        const std::string_view& path, std::vector<TypedParam>& params,
        std::vector<TypedParam>& splats) const
    {
        // recursion to correct path segment
        if (!path.empty())
        {
            const auto segment_delimiter = path.find('/');
            // current segment value
            auto current_segment = path.substr(0, segment_delimiter);
            // complete child path (path without this segment)
            // if no '/' was found, it means that it is a leaf resource
            const auto lower_path = (segment_delimiter == std::string_view::npos)
                ? std::string_view { nullptr, 0 }
                : path.substr(segment_delimiter + 1);

            // Check if it is a fixed route
            if (fixed_.count(current_segment) != 0)
            {
                auto result = fixed_.at(current_segment)->findRoute(lower_path, params, splats);
                auto route  = std::get<0>(result);
                if (route != nullptr)
                    return result;
            }

            // Check if it is a path param
            for (const auto& param : param_)
            {
                std::string para_name { param.first.data(), param.first.length() };
                std::string para_val { current_segment.data(), current_segment.length() };
                params.emplace_back(para_name, para_val);
                auto result = param.second->findRoute(lower_path, params, splats);
                auto route  = std::get<0>(result);
                if (route != nullptr)
                    return result;
                params.pop_back();
            }

            // Check if it is an optional path param
            for (const auto& optional : optional_)
            {
                std::string opt_name { optional.first.data(), optional.first.length() };
                std::string opt_val { current_segment.data(), current_segment.length() };
                params.emplace_back(opt_name, opt_val);
                auto result = optional.second->findRoute(lower_path, params, splats);

                auto route = std::get<0>(result);
                if (route != nullptr)
                    return result;
                params.pop_back();
                // try to find a route for lower path assuming that
                // this optional path param is not present
                result = optional.second->findRoute(lower_path, params, splats);

                route = std::get<0>(result);
                if (route != nullptr)
                    return result;
            }

            // Check if it is a splat
            if (splat_ != nullptr)
            {
                std::string splat { current_segment.data(), current_segment.length() };
                splats.emplace_back(splat, splat);
                auto result = splat_->findRoute(lower_path, params, splats);

                auto route = std::get<0>(result);
                if (route != nullptr)
                    return result;
                splats.pop_back();
            }
            // Requested route does not exists
            return std::make_tuple(nullptr, std::vector<TypedParam>(),
                                   std::vector<TypedParam>());
        }
        else
        { // current leaf requested, or empty final optional
            if (!optional_.empty())
            {
                // in case of more than one optional at this point, as it is an
                // ambiguity, it is resolved by using the first optional
                auto optional = optional_.begin();
                // std::string opt {optional->first.data(), optional->first.length()};
                return optional->second->findRoute(path, params, splats);
            }
            else if (route_ == nullptr)
            {
                // if we are here but route is null, we reached this point
                // trying to parse an optional, that was missing
                return std::make_tuple(nullptr, std::vector<TypedParam>(),
                                       std::vector<TypedParam>());
            }
            else
            {
                return std::make_tuple(route_, std::move(params), std::move(splats));
            }
        }
    }

    std::tuple<std::shared_ptr<Route>, std::vector<TypedParam>,
               std::vector<TypedParam>>
    Pistache::Rest::SegmentTreeNode::findRoute(const std::string_view& path) const
    {
        std::vector<TypedParam> params;
        std::vector<TypedParam> splats;
        return findRoute(path, params, splats);
    }

    namespace Private
    {

        RouterHandler::RouterHandler(const Rest::Router& router)
            : router(std::make_shared<Rest::Router>(router))
        { }

        RouterHandler::RouterHandler(std::shared_ptr<Rest::Router> router)
            : router(std::move(router))
        { }

        void RouterHandler::onRequest(const Http::Request& req,
                                      Http::ResponseWriter response)
        {
            router->route(req, std::move(response));
        }

        void RouterHandler::onDisconnection(const std::shared_ptr<Tcp::Peer>& peer)
        {
            router->disconnectPeer(peer);
        }

    } // namespace Private

    Router Router::fromDescription(const Rest::Description& desc)
    {
        Router router;
        router.initFromDescription(desc);
        return router;
    }

    std::shared_ptr<Private::RouterHandler> Router::handler() const
    {
        return std::make_shared<Private::RouterHandler>(*this);
    }

    std::shared_ptr<Private::RouterHandler>
    Router::handler(std::shared_ptr<Rest::Router> router)
    {
        return std::make_shared<Private::RouterHandler>(router);
    }

    void Router::initFromDescription(const Rest::Description& desc)
    {
        const auto& paths = desc.rawPaths();
        for (auto it = paths.flatBegin(), end = paths.flatEnd(); it != end; ++it)
        {
            const auto& paths_ = *it;
            for (const auto& path : paths_)
            {
                if (!path.isBound())
                {
                    std::ostringstream oss;
                    oss << "Path '" << path.value << "' is not bound";
                    throw std::runtime_error(oss.str());
                }

                addRoute(path.method, path.value, path.handler);
            }
        }
    }

    void Router::get(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Get, resource, std::move(handler));
    }

    void Router::post(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Post, resource, std::move(handler));
    }

    void Router::put(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Put, resource, std::move(handler));
    }

    void Router::patch(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Patch, resource, std::move(handler));
    }

    void Router::del(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Delete, resource, std::move(handler));
    }

    void Router::options(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Options, resource, std::move(handler));
    }

    void Router::removeRoute(Http::Method method, const std::string& resource)
    {
        if (resource.empty())
            throw std::runtime_error("Invalid zero-length URL.");
        auto& r              = routes[method];
        const auto sanitized = SegmentTreeNode::sanitizeResource(resource);
        const std::string_view path { sanitized.data(), sanitized.size() };
        r.removeRoute(path);
    }

    void Router::head(const std::string& resource, Route::Handler handler)
    {
        addRoute(Http::Method::Head, resource, std::move(handler));
    }

    void Router::addCustomHandler(Route::Handler handler)
    {
        customHandlers.push_back(std::move(handler));
    }

    void Router::addMiddleware(Route::Middleware middleware)
    {
        middlewares.push_back(std::move(middleware));
    }

    void Router::addDisconnectHandler(Route::DisconnectHandler handler)
    {
        disconnectHandlers.push_back(std::move(handler));
    }

    void Router::addNotFoundHandler(Route::Handler handler)
    {
        notFoundHandler = std::move(handler);
    }

    void Router::invokeNotFoundHandler(const Http::Request& req,
                                       Http::ResponseWriter resp) const
    {
        notFoundHandler(Rest::Request(std::move(req), std::vector<TypedParam>(),
                                      std::vector<TypedParam>()),
                        std::move(resp));
    }

    Route::Status Router::route(const Http::Request& request,
                                Http::ResponseWriter response)
    {
        const auto& resource = request.resource();
        if (resource.empty())
            throw std::runtime_error("Invalid zero-length URL.");

        auto req  = request;
        auto resp = response.clone();

        for (const auto& middleware : middlewares)
        {
            auto result = middleware(req, resp);

            // Handler returns true, go to the next piped handler, otherwise break and return
            if (!result)
                return Route::Status::Match;
        }

        auto& r              = routes[req.method()];
        const auto sanitized = SegmentTreeNode::sanitizeResource(resource);
        const std::string_view path { sanitized.data(), sanitized.size() };
        auto result = r.findRoute(path);

        auto route = std::get<0>(result);
        if (route != nullptr)
        {
            auto params = std::get<1>(result);
            auto splats = std::get<2>(result);
            route->invokeHandler(Request(std::move(req), std::move(params), std::move(splats)),
                                 std::move(resp));
            return Route::Status::Match;
        }

        for (const auto& handler : customHandlers)
        {
            auto resp     = response.clone();
            auto handler1 = handler(
                Request(req, std::vector<TypedParam>(), std::vector<TypedParam>()),
                std::move(resp));
            if (handler1 == Route::Result::Ok)
                return Route::Status::Match;
        }

        // No route or custom handler found. Let's walk through the
        // list of other methods and see if any of them support
        // this resource.
        // This will allow server to send a
        // HTTP 405 (method not allowed) response.
        // RFC 7231 requires HTTP 405 responses to include a list of
        // supported methods for the requested resource.
        std::vector<Http::Method> supportedMethods;
        for (auto& methods : routes)
        {
            if (methods.first == req.method())
                continue;

            auto res = methods.second.findRoute(path);
            auto rte = std::get<0>(res);
            if (rte != nullptr)
            {
                supportedMethods.push_back(methods.first);
            }
        }

        if (!supportedMethods.empty())
        {
            response.sendMethodNotAllowed(supportedMethods);
            return Route::Status::NotAllowed;
        }

        if (hasNotFoundHandler())
        {
            invokeNotFoundHandler(req, std::move(response));
        }
        else
        {
            response.send(Http::Code::Not_Found, "Could not find a matching route");
        }
        return Route::Status::NotFound;
    }

    void Router::addRoute(Http::Method method, const std::string& resource,
                          Route::Handler handler)
    {
        if (resource.empty())
            throw std::runtime_error("Invalid zero-length URL.");
        auto& r              = routes[method];
        const auto sanitized = SegmentTreeNode::sanitizeResource(resource);
        std::shared_ptr<char> ptr(new char[sanitized.length()],
                                  std::default_delete<char[]>());
        memcpy(ptr.get(), sanitized.data(), sanitized.length());
        const std::string_view path { ptr.get(), sanitized.length() };
        r.addRoute(path, handler, ptr);
    }

    void Router::disconnectPeer(const std::shared_ptr<Tcp::Peer>& peer)
    {
        for (const auto& handler : disconnectHandlers)
        {
            handler(peer);
        }
    }

    namespace Routes
    {

        void Get(Router& router, const std::string& resource, Route::Handler handler)
        {
            router.get(resource, std::move(handler));
        }

        void Post(Router& router, const std::string& resource, Route::Handler handler)
        {
            router.post(resource, std::move(handler));
        }

        void Put(Router& router, const std::string& resource, Route::Handler handler)
        {
            router.put(resource, std::move(handler));
        }

        void Patch(Router& router, const std::string& resource,
                   Route::Handler handler)
        {
            router.patch(resource, std::move(handler));
        }

        void Delete(Router& router, const std::string& resource,
                    Route::Handler handler)
        {
            router.del(resource, std::move(handler));
        }

        void Options(Router& router, const std::string& resource,
                     Route::Handler handler)
        {
            router.options(resource, std::move(handler));
        }

        void Remove(Router& router, Http::Method method, const std::string& resource)
        {
            router.removeRoute(method, resource);
        }

        void NotFound(Router& router, Route::Handler handler)
        {
            router.addNotFoundHandler(std::move(handler));
        }

        void Head(Router& router, const std::string& resource, Route::Handler handler)
        {
            router.head(resource, std::move(handler));
        }

    } // namespace Routes
} // namespace Pistache::Rest
