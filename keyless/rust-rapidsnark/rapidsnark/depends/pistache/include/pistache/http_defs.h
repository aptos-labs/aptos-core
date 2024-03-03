/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* http_defs.h
   Mathieu Stefani, 01 September 2015

   Various http definitions
*/

#pragma once

#include <chrono>
#include <functional>
#include <ostream>
#include <stdexcept>
#include <string>

namespace Pistache::Http
{

#define HTTP_METHODS                               \
    METHOD(Options, "OPTIONS")                     \
    METHOD(Get, "GET")                             \
    METHOD(Post, "POST")                           \
    METHOD(Head, "HEAD")                           \
    METHOD(Put, "PUT")                             \
    METHOD(Patch, "PATCH")                         \
    METHOD(Delete, "DELETE")                       \
    METHOD(Trace, "TRACE")                         \
    METHOD(Connect, "CONNECT")                     \
    METHOD(Acl, "ACL")                             \
    METHOD(BaselineControl, "BASELINE-CONTROL")    \
    METHOD(Bind, "BIND")                           \
    METHOD(Checkin, "CHECKIN")                     \
    METHOD(Checkout, "CHECKOUT")                   \
    METHOD(Copy, "COPY")                           \
    METHOD(Label, "LABEL")                         \
    METHOD(Link, "LINK")                           \
    METHOD(Lock, "LOCK")                           \
    METHOD(Merge, "MERGE")                         \
    METHOD(Mkactivity, "MKACTIVITY")               \
    METHOD(Mkcalendar, "MKCALENDAR")               \
    METHOD(Mkcol, "MKCOL")                         \
    METHOD(Mkredirectref, "MKREDIRECTREF")         \
    METHOD(Mkworkspace, "MKWORKSPACE")             \
    METHOD(Move, "MOVE")                           \
    METHOD(Orderpatch, "ORDERPATCH")               \
    METHOD(Pri, "PRI")                             \
    METHOD(Propfind, "PROPFIND")                   \
    METHOD(Proppatch, "PROPPATCH")                 \
    METHOD(Rebind, "REBIND")                       \
    METHOD(Report, "REPORT")                       \
    METHOD(Search, "SEARCH")                       \
    METHOD(Unbind, "UNBIND")                       \
    METHOD(Uncheckout, "UNCHECKOUT")               \
    METHOD(Unlink, "UNLINK")                       \
    METHOD(Unlock, "UNLOCK")                       \
    METHOD(Update, "UPDATE")                       \
    METHOD(Updateredirectref, "UPDATEREDIRECTREF") \
    METHOD(VersionControl, "VERSION-CONTROL")

// 10. Status Code Definitions
#define STATUS_CODES                                                          \
    CODE(100, Continue, "Continue")                                           \
    CODE(101, Switching_Protocols, "Switching Protocols")                     \
    CODE(102, Processing, "Processing")                                       \
    CODE(103, Early_Hints, "Early Hints")                                     \
    CODE(200, Ok, "OK")                                                       \
    CODE(201, Created, "Created")                                             \
    CODE(202, Accepted, "Accepted")                                           \
    CODE(203, NonAuthoritative_Information, "Non-Authoritative Information")  \
    CODE(204, No_Content, "No Content")                                       \
    CODE(205, Reset_Content, "Reset Content")                                 \
    CODE(206, Partial_Content, "Partial Content")                             \
    CODE(207, MultiStatus, "Multi-Status")                                    \
    CODE(208, Already_Reported, "Already Reported")                           \
    CODE(226, IM_Used, "IM Used")                                             \
    CODE(300, Multiple_Choices, "Multiple Choices")                           \
    CODE(301, Moved_Permanently, "Moved Permanently")                         \
    CODE(302, Found, "Found")                                                 \
    CODE(303, See_Other, "See Other")                                         \
    CODE(304, Not_Modified, "Not Modified")                                   \
    CODE(305, Use_Proxy, "Use Proxy")                                         \
    CODE(307, Temporary_Redirect, "Temporary Redirect")                       \
    CODE(308, Permanent_Redirect, "Permanent Redirect")                       \
    CODE(400, Bad_Request, "Bad Request")                                     \
    CODE(401, Unauthorized, "Unauthorized")                                   \
    CODE(402, Payment_Required, "Payment Required")                           \
    CODE(403, Forbidden, "Forbidden")                                         \
    CODE(404, Not_Found, "Not Found")                                         \
    CODE(405, Method_Not_Allowed, "Method Not Allowed")                       \
    CODE(406, Not_Acceptable, "Not Acceptable")                               \
    CODE(407, Proxy_Authentication_Required, "Proxy Authentication Required") \
    CODE(408, Request_Timeout, "Request Timeout")                             \
    CODE(409, Conflict, "Conflict")                                           \
    CODE(410, Gone, "Gone")                                                   \
    CODE(411, Length_Required, "Length Required")                             \
    CODE(412, Precondition_Failed, "Precondition Failed")                     \
    CODE(413, Request_Entity_Too_Large, "Request Entity Too Large")           \
    CODE(414, RequestURI_Too_Long, "Request-URI Too Long")                    \
    CODE(415, Unsupported_Media_Type, "Unsupported Media Type")               \
    CODE(416, Requested_Range_Not_Satisfiable,                                \
         "Requested Range Not Satisfiable")                                   \
    CODE(417, Expectation_Failed, "Expectation Failed")                       \
    CODE(418, I_m_a_teapot, "I'm a teapot")                                   \
    CODE(421, Misdirected_Request, "Misdirected Request")                     \
    CODE(422, Unprocessable_Entity, "Unprocessable Entity")                   \
    CODE(423, Locked, "Locked")                                               \
    CODE(424, Failed_Dependency, "Failed Dependency")                         \
    CODE(426, Upgrade_Required, "Upgrade Required")                           \
    CODE(428, Precondition_Required, "Precondition Required")                 \
    CODE(429, Too_Many_Requests, "Too Many Requests")                         \
    CODE(431, Request_Header_Fields_Too_Large,                                \
         "Request Header Fields Too Large")                                   \
    CODE(444, Connection_Closed_Without_Response,                             \
         "Connection Closed Without Response")                                \
    CODE(451, Unavailable_For_Legal_Reasons, "Unavailable For Legal Reasons") \
    CODE(499, Client_Closed_Request, "Client Closed Request")                 \
    CODE(500, Internal_Server_Error, "Internal Server Error")                 \
    CODE(501, Not_Implemented, "Not Implemented")                             \
    CODE(502, Bad_Gateway, "Bad Gateway")                                     \
    CODE(503, Service_Unavailable, "Service Unavailable")                     \
    CODE(504, Gateway_Timeout, "Gateway Timeout")                             \
    CODE(505, HTTP_Version_Not_Supported, "HTTP Version Not Supported")       \
    CODE(506, Variant_Also_Negotiates, "Variant Also Negotiates")             \
    CODE(507, Insufficient_Storage, "Insufficient Storage")                   \
    CODE(508, Loop_Detected, "Loop Detected")                                 \
    CODE(510, Not_Extended, "Not Extended")                                   \
    CODE(511, Network_Authentication_Required,                                \
         "Network Authentication Required")                                   \
    CODE(599, Network_Connect_Timeout_Error, "Network Connect Timeout Error")

// 3.4. Character Sets
// See http://tools.ietf.org/html/rfc2978 and
// http://www.iana.org/assignments/character-sets/character-sets.xhtml
#define CHARSETS                            \
    CHARSET(UsAscii, "us-ascii")            \
    CHARSET(Iso - 8859 - 1, "iso-8859-1")   \
    CHARSET(Iso - 8859 - 2, "iso-8859-2")   \
    CHARSET(Iso - 8859 - 3, "iso-8859-3")   \
    CHARSET(Iso - 8859 - 4, "iso-8859-4")   \
    CHARSET(Iso - 8859 - 5, "iso-8859-5")   \
    CHARSET(Iso - 8859 - 6, "iso-8859-6")   \
    CHARSET(Iso - 8859 - 7, "iso-8859-7")   \
    CHARSET(Iso - 8859 - 8, "iso-8859-8")   \
    CHARSET(Iso - 8859 - 9, "iso-8859-9")   \
    CHARSET(Iso - 8859 - 10, "iso-8859-10") \
    CHARSET(Shift - JIS, "shift_jis")       \
    CHARSET(Utf7, "utf-7")                  \
    CHARSET(Utf8, "utf-8")                  \
    CHARSET(Utf16, "utf-16")                \
    CHARSET(Utf16 - BE, "utf-16be")         \
    CHARSET(Utf16 - LE, "utf-16le")         \
    CHARSET(Utf32, "utf-32")                \
    CHARSET(Utf32 - BE, "utf-32be")         \
    CHARSET(Utf32 - LE, "utf-32le")         \
    CHARSET(Unicode - 11, "unicode-1-1")

    enum class Method {
#define METHOD(m, _) m,
        HTTP_METHODS
#undef METHOD
    };

    enum class Code {
#define CODE(value, name, _) name = value,
        STATUS_CODES
#undef CODE
    };

    enum class Version {
        Http10, // HTTP/1.0
        Http11 // HTTP/1.1
    };

    enum class ConnectionControl { Close,
                                   KeepAlive,
                                   Ext };

    enum class Expectation { Continue,
                             Ext };

    class CacheDirective
    {
    public:
        enum Directive {
            NoCache,
            NoStore,
            MaxAge,
            MaxStale,
            MinFresh,
            NoTransform,
            OnlyIfCached,
            Public,
            Private,
            MustRevalidate,
            ProxyRevalidate,
            SMaxAge,
            Ext
        };

        CacheDirective()
            : directive_()
            , data()
        { }

        explicit CacheDirective(Directive directive);
        CacheDirective(Directive directive, std::chrono::seconds delta);

        Directive directive() const { return directive_; }
        std::chrono::seconds delta() const;

    private:
        void init(Directive directive, std::chrono::seconds delta);
        Directive directive_;
        // Poor way of representing tagged unions in C++
        union
        {
            uint64_t maxAge;
            uint64_t sMaxAge;
            uint64_t maxStale;
            uint64_t minFresh;
        } data;
    };

    // 3.3.1 Full Date
    class FullDate
    {
    public:
        using time_point = std::chrono::system_clock::time_point;
        FullDate()
            : date_()
        { }

        enum class Type { RFC1123,
                          RFC850,
                          AscTime };

        explicit FullDate(time_point date)
            : date_(date)
        { }

        time_point date() const { return date_; }
        void write(std::ostream& os, Type type = Type::RFC1123) const;

        static FullDate fromString(const std::string& str);

    private:
        time_point date_;
    };

    const char* methodString(Method method);
    const char* versionString(Version version);
    const char* codeString(Code code);

    std::ostream& operator<<(std::ostream& os, Version version);
    std::ostream& operator<<(std::ostream& os, Method method);
    std::ostream& operator<<(std::ostream& os, Code code);

    struct HttpError : public std::exception
    {
        HttpError(Code code, std::string reason);
        HttpError(int code, std::string reason);

        ~HttpError() noexcept override = default;

        const char* what() const noexcept override { return reason_.c_str(); }

        int code() const { return code_; }
        std::string reason() const { return reason_; }

    private:
        int code_;
        std::string reason_;
    };

} // namespace Pistache::Http

namespace std
{

    template <>
    struct hash<Pistache::Http::Method>
    {
        size_t operator()(Pistache::Http::Method method) const
        {
            return std::hash<int>()(static_cast<int>(method));
        }
    };

} // namespace std
