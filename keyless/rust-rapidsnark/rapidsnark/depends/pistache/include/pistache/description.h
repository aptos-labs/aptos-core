/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 24 février 2016

   An API description (reflection) mechanism that is based on Swagger
*/

#pragma once

#include <algorithm>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <type_traits>
#include <vector>

#include <pistache/http_defs.h>
#include <pistache/iterator_adapter.h>
#include <pistache/mime.h>
#include <pistache/router.h>

namespace Pistache::Rest
{
    namespace Type
    {

        // Data Types

#define DATA_TYPE                                               \
    TYPE(Integer, std::int32_t, "integer", "int32")             \
    TYPE(Long, std::int64_t, "integer", "int64")                \
    TYPE(Float, float, "number", "float")                       \
    TYPE(Double, double, "number", "double")                    \
    TYPE(String, std::string, "string", "")                     \
    TYPE(Byte, char, "string", "byte")                          \
    TYPE(Binary, std::vector<std::uint8_t>, "string", "binary") \
    TYPE(Bool, bool, "boolean", "")                             \
    COMPLEX_TYPE(Date, "string", "date")                        \
    COMPLEX_TYPE(Datetime, "string", "date-time")               \
    COMPLEX_TYPE(Password, "string", "password")                \
    COMPLEX_TYPE(Array, "array", "array")

#define TYPE(rest, cpp, _, __) typedef cpp rest;
#define COMPLEX_TYPE(rest, _, __) \
    struct rest                   \
    { };
        DATA_TYPE
#undef TYPE
#undef COMPLEX_TYPE

    } // namespace Type

#define SCHEMES            \
    SCHEME(Http, "http")   \
    SCHEME(Https, "https") \
    SCHEME(Ws, "ws")       \
    SCHEME(Wss, "wss")

    enum class Scheme {
#define SCHEME(e, _) e,
        SCHEMES
#undef SCHEME
    };

    const char* schemeString(Scheme scheme);

    namespace Schema
    {

        namespace Traits
        {

            template <typename DT>
            struct IsDataType : public std::false_type
            { };

#define TYPE(rest, _, __, ___)                            \
    template <>                                           \
    struct IsDataType<Type::rest> : public std::true_type \
    { };
#define COMPLEX_TYPE(rest, _, __)                         \
    template <>                                           \
    struct IsDataType<Type::rest> : public std::true_type \
    { };
            DATA_TYPE
#undef TYPE
#undef COMPLEX_TYPE

            template <typename DT>
            struct DataTypeInfo;

#define TYPE(rest, _, typeStr, formatStr)                 \
    template <>                                           \
    struct DataTypeInfo<Type::rest>                       \
    {                                                     \
        static const char* typeName() { return typeStr; } \
        static const char* format() { return formatStr; } \
    };
#define COMPLEX_TYPE(rest, typeStr, formatStr)            \
    template <>                                           \
    struct DataTypeInfo<Type::rest>                       \
    {                                                     \
        static const char* typeName() { return typeStr; } \
        static const char* format() { return formatStr; } \
    };
            DATA_TYPE
#undef TYPE
#undef COMPLEX_TYPE

            template <typename DT>
            struct DataTypeValidation
            {
                static bool validate(const std::string&) { return true; }
            };

        } // namespace Traits

        struct ProduceConsume
        {
            ProduceConsume()
                : produce()
                , consume()
            { }

            std::vector<Http::Mime::MediaType> produce;
            std::vector<Http::Mime::MediaType> consume;
        };

        struct Contact
        {
            Contact(std::string name, std::string url, std::string email);

            std::string name;
            std::string url;
            std::string email;
        };

        struct License
        {
            License(std::string name, std::string url);

            std::string name;
            std::string url;
        };

        struct Info
        {
            Info(std::string title, std::string version, std::string description = "");

            std::string title;
            std::string version;
            std::string description;
            std::string termsOfService;

            std::optional<Contact> contact;
            std::optional<License> license;
        };

        struct InfoBuilder
        {
            explicit InfoBuilder(Info* info);

            InfoBuilder& termsOfService(std::string value);
            InfoBuilder& contact(std::string name, std::string url, std::string email);
            InfoBuilder& license(std::string name, std::string url);

        private:
            Info* info_;
        };

        struct DataType
        {
            virtual const char* typeName() const = 0;
            virtual const char* format() const   = 0;

            virtual bool validate(const std::string& input) const = 0;

            virtual ~DataType() = default;
        };

        template <typename T>
        struct DataTypeT : public DataType
        {
            const char* typeName() const override
            {
                return Traits::DataTypeInfo<T>::typeName();
            }
            const char* format() const override
            {
                return Traits::DataTypeInfo<T>::format();
            }

            bool validate(const std::string& input) const override
            {
                return Traits::DataTypeValidation<T>::validate(input);
            }

            ~DataTypeT() override = default;
        };

        template <typename T>
        std::unique_ptr<DataType> makeDataType()
        {
            static_assert(Traits::IsDataType<T>::value, "Unknown Data Type");
            return std::unique_ptr<DataType>(new DataTypeT<T>());
        }

        struct Parameter
        {
            Parameter(std::string name, std::string description);

            template <typename T, typename... Args>
            static Parameter create(Args&&... args)
            {
                Parameter p(std::forward<Args>(args)...);
                p.type = makeDataType<T>();
                return p;
            }

            std::string name;
            std::string description;
            bool required;
            std::shared_ptr<DataType> type;
        };

        struct Response
        {
            Response(Http::Code statusCode, std::string description);

            Http::Code statusCode;
            std::string description;
        };

        struct ResponseBuilder
        {
            ResponseBuilder(Http::Code statusCode, std::string description);

            operator Response() const { return response_; }

        private:
            Response response_;
        };

        struct PathDecl
        {
            PathDecl(std::string value, Http::Method method);

            std::string value;
            Http::Method method;
        };

        struct Path
        {
            Path(std::string value, Http::Method method, std::string description);

            std::string value;
            Http::Method method;
            std::string description;
            bool hidden;

            ProduceConsume pc;
            std::vector<Parameter> parameters;
            std::vector<Response> responses;

            Route::Handler handler;

            static std::string swaggerFormat(const std::string& path);

            bool isBound() const { return handler != nullptr; }
        };

        class PathGroup
        {
        public:
            struct Group : public std::vector<Path>
            {
                bool isHidden() const
                {
                    if (empty())
                        return false;

                    return std::all_of(begin(), end(),
                                       [](const Path& path) { return path.hidden; });
                }
            };

            typedef std::unordered_map<std::string, Group> Map;
            typedef Map::iterator iterator;
            typedef Map::const_iterator const_iterator;

            typedef std::vector<Path>::iterator group_iterator;

            typedef FlatMapIteratorAdapter<Map> flat_iterator;

            enum class Format { Default,
                                Swagger };

            bool hasPath(const std::string& name, Http::Method method) const;
            bool hasPath(const Path& path) const;

            PathGroup()
                : groups_()
            { }

            Group paths(const std::string& name) const;
            std::optional<Path> path(const std::string& name, Http::Method method) const;

            group_iterator add(Path path);

            template <typename... Args>
            group_iterator emplace(Args&&... args)
            {
                return add(Path(std::forward<Args>(args)...));
            }

            const_iterator begin() const;
            const_iterator end() const;

            flat_iterator flatBegin() const;
            flat_iterator flatEnd() const;

            Map groups() const { return groups_; }

        private:
            Map groups_;
        };

        struct PathBuilder
        {
            explicit PathBuilder(Path* path);

            template <typename... Mimes>
            PathBuilder& produces(Mimes... mimes)
            {
                Http::Mime::MediaType m[sizeof...(Mimes)] = { mimes... };
                std::copy(std::begin(m), std::end(m),
                          std::back_inserter(path_->pc.produce));
                return *this;
            }

            template <typename... Mimes>
            PathBuilder& consumes(Mimes... mimes)
            {
                Http::Mime::MediaType m[sizeof...(Mimes)] = { mimes... };
                std::copy(std::begin(m), std::end(m),
                          std::back_inserter(path_->pc.consume));
                return *this;
            }

            template <typename T>
            PathBuilder& parameter(std::string name, std::string description)
            {
                path_->parameters.push_back(
                    Parameter::create<T>(std::move(name), std::move(description)));
                return *this;
            }

            PathBuilder& response(Http::Code statusCode, std::string description)
            {
                path_->responses.emplace_back(statusCode, std::move(description));
                return *this;
            }

            PathBuilder& response(Response response)
            {
                path_->responses.push_back(std::move(response));
                return *this;
            }

            /* @CodeDup: should re-use Routes::bind */
            template <typename Result, typename Cls, typename... Args, typename Obj>
            PathBuilder& bind(Result (Cls::*func)(Args...), Obj obj)
            {

#define CALL_MEMBER_FN(obj, pmf) ((obj)->*(pmf))

                path_->handler = [=](const Rest::Request& request,
                                     Http::ResponseWriter response) {
                    CALL_MEMBER_FN(obj, func)
                    (request, std::move(response));

                    return Route::Result::Ok;
                };

#undef CALL_MEMBER_FN

                return *this;
            }

            template <typename Result, typename... Args>
            PathBuilder& bind(Result (*func)(Args...))
            {

                path_->handler = [=](const Rest::Request& request,
                                     Http::ResponseWriter response) {
                    func(request, std::move(response));

                    return Route::Result::Ok;
                };

                return *this;
            }

            PathBuilder& hide(bool value = true)
            {
                path_->hidden = value;
                return *this;
            }

        private:
            Path* path_;
        };

        struct SubPath
        {
            SubPath(std::string prefix, PathGroup* paths);

            PathBuilder route(const std::string& name, Http::Method method,
                              std::string description = "");
            PathBuilder route(PathDecl fragment, std::string description = "");

            SubPath path(const std::string& prefix) const;

            template <typename T>
            void parameter(std::string name, std::string description)
            {
                parameters.push_back(
                    Parameter::create<T>(std::move(name), std::move(description)));
            }

            std::string prefix;
            std::vector<Parameter> parameters;
            PathGroup* paths;
        };

    } // namespace Schema

    class Description
    {
    public:
        Description(std::string title, std::string version,
                    std::string description = "");

        Schema::InfoBuilder info();

        Description& host(std::string value);
        Description& basePath(std::string value);

        template <typename... Schemes>
        Description& schemes(Schemes... _schemes)
        {
            Scheme s[sizeof...(Schemes)] = { _schemes... };
            std::copy(std::begin(s), std::end(s), std::back_inserter(schemes_));
            return *this;
        }

        template <typename... Mimes>
        Description& produces(Mimes... mimes)
        {
            Http::Mime::MediaType m[sizeof...(Mimes)] = { mimes... };
            std::copy(std::begin(m), std::end(m), std::back_inserter(pc.produce));
            return *this;
        }

        template <typename... Mimes>
        Description& consumes(Mimes... mimes)
        {
            Http::Mime::MediaType m[sizeof...(Mimes)] = { mimes... };
            std::copy(std::begin(m), std::end(m), std::back_inserter(pc.consume));
            return *this;
        }

        Schema::PathDecl options(std::string name);
        Schema::PathDecl get(std::string name);
        Schema::PathDecl post(std::string name);
        Schema::PathDecl head(std::string name);
        Schema::PathDecl put(std::string name);
        Schema::PathDecl patch(std::string name);
        Schema::PathDecl del(std::string name);
        Schema::PathDecl trace(std::string name);
        Schema::PathDecl connect(std::string name);

        Schema::SubPath path(std::string name);

        Schema::PathBuilder route(std::string name, Http::Method method,
                                  std::string description = "");
        Schema::PathBuilder route(Schema::PathDecl fragment,
                                  std::string description = "");

        Schema::ResponseBuilder response(Http::Code statusCode,
                                         std::string description);

        Schema::Info rawInfo() const { return info_; }
        std::string rawHost() const { return host_; }
        std::string rawBasePath() const { return basePath_; }
        std::vector<Scheme> rawSchemes() const { return schemes_; }
        Schema::ProduceConsume rawPC() const { return pc; }
        const Schema::PathGroup& rawPaths() const { return paths_; }

    private:
        Schema::Info info_;
        std::string host_;
        std::string basePath_;
        std::vector<Scheme> schemes_;
        Schema::ProduceConsume pc;

        Schema::PathGroup paths_;
    };

    class Swagger
    {
    public:
        explicit Swagger(const Description& description)
            : description_(description)
            , uiPath_()
            , uiDirectory_()
            , apiPath_()
            , serializer_()
        { }

        typedef std::function<std::string(const Description&)> Serializer;

        Swagger& uiPath(std::string path);
        Swagger& uiDirectory(std::string dir);
        Swagger& apiPath(std::string path);
        Swagger& serializer(Serializer serialize);

        void install(Rest::Router& router);

    private:
        Description description_;
        std::string uiPath_;
        std::string uiDirectory_;
        std::string apiPath_;
        Serializer serializer_;
    };

} // namespace Pistache::Rest
