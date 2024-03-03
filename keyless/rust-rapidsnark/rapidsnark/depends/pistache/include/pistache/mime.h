/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* mime.h
   Mathieu Stefani, 29 August 2015

   Type safe representation of a MIME Type (RFC 1590)
*/

#pragma once

#include <cassert>
#include <cmath>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>

namespace Pistache::Http::Mime
{

#define MIME_TYPES                   \
    TYPE(Star, "*")                  \
    TYPE(Text, "text")               \
    TYPE(Image, "image")             \
    TYPE(Audio, "audio")             \
    TYPE(Video, "video")             \
    TYPE(Application, "application") \
    TYPE(Message, "message")         \
    TYPE(Multipart, "multipart")

#define MIME_SUBTYPES                                    \
    SUB_TYPE(Star, "*")                                  \
    SUB_TYPE(Plain, "plain")                             \
    SUB_TYPE(Html, "html")                               \
    SUB_TYPE(Xhtml, "xhtml")                             \
    SUB_TYPE(Xml, "xml")                                 \
    SUB_TYPE(Javascript, "javascript")                   \
    SUB_TYPE(Css, "css")                                 \
                                                         \
    SUB_TYPE(OctetStream, "octet-stream")                \
    SUB_TYPE(Json, "json")                               \
    SUB_TYPE(JsonSchema, "schema+json")                  \
    SUB_TYPE(JsonSchemaInstance, "schema-instance+json") \
    SUB_TYPE(FormUrlEncoded, "x-www-form-urlencoded")    \
    SUB_TYPE(FormData, "form-data")                      \
                                                         \
    SUB_TYPE(Png, "png")                                 \
    SUB_TYPE(Gif, "gif")                                 \
    SUB_TYPE(Bmp, "bmp")                                 \
    SUB_TYPE(Jpeg, "jpeg")

#define MIME_SUFFIXES                                  \
    SUFFIX(Json, "json", "JavaScript Object Notation") \
    SUFFIX(Ber, "ber", "Basic Encoding Rules")         \
    SUFFIX(Der, "der", "Distinguished Encoding Rules") \
    SUFFIX(Fastinfoset, "fastinfoset", "Fast Infoset") \
    SUFFIX(Wbxml, "wbxml", "WAP Binary XML")           \
    SUFFIX(Zip, "zip", "ZIP file storage")             \
    SUFFIX(Xml, "xml", "Extensible Markup Language")

    enum class Type {
#define TYPE(val, _) val,
        MIME_TYPES
#undef TYPE
            None
    };

    enum class Subtype {
#define SUB_TYPE(val, _) val,
        MIME_SUBTYPES
#undef SUB_TYPE
            Vendor,
        Ext,
        None
    };

    enum class Suffix {
#define SUFFIX(val, _, __) val,
        MIME_SUFFIXES
#undef SUFFIX
            None,
        Ext
    };

    // 3.9 Quality Values
    class Q
    {
    public:
        // typedef uint8_t Type;

        typedef uint16_t Type;

        explicit Q(Type val)
            : val_()
        {
            if (val > 100)
            {
                throw std::runtime_error(
                    "Invalid quality value, must be in the [0; 100] range");
            }

            val_ = val;
        }

        static Q fromFloat(double f)
        {
            return Q(static_cast<Type>(round(f * 100.0)));
        }

        Type value() const { return val_; }
        operator Type() const { return val_; }

        std::string toString() const;

    private:
        Type val_;
    };

    inline bool operator==(Q lhs, Q rhs) { return lhs.value() == rhs.value(); }

    // 3.7 Media Types
    class MediaType
    {
    public:
        enum Parse { DoParse,
                     DontParse };

        MediaType()
            : top_(Type::None)
            , sub_(Subtype::None)
            , suffix_(Suffix::None)
            , raw_()
            , rawSubIndex()
            , rawSuffixIndex()
            , params()
            , q_()
        { }

        explicit MediaType(std::string raw, Parse parse = DontParse)
            : top_(Type::None)
            , sub_(Subtype::None)
            , suffix_(Suffix::None)
            , raw_()
            , rawSubIndex()
            , rawSuffixIndex()
            , params()
            , q_()
        {
            if (parse == DoParse)
            {
                parseRaw(raw.c_str(), raw.length());
            }
            else
            {
                raw_ = std::move(raw);
            }
        }

        MediaType(Mime::Type top, Mime::Subtype sub)
            : top_(top)
            , sub_(sub)
            , suffix_(Suffix::None)
            , raw_()
            , rawSubIndex()
            , rawSuffixIndex()
            , params()
            , q_()
        { }

        MediaType(Mime::Type top, Mime::Subtype sub, Mime::Suffix suffix)
            : top_(top)
            , sub_(sub)
            , suffix_(suffix)
            , raw_()
            , rawSubIndex()
            , rawSuffixIndex()
            , params()
            , q_()
        { }

        void parseRaw(const char* str, size_t len);
        static MediaType fromRaw(const char* str, size_t len);

        static MediaType fromString(const std::string& str);
        static MediaType fromString(std::string&& str);

        static MediaType fromFile(const char* fileName);

        Mime::Type top() const { return top_; }
        Mime::Subtype sub() const { return sub_; }
        Mime::Suffix suffix() const { return suffix_; }

        std::string rawSub() const { return rawSubIndex.splice(raw_); }

        std::string raw() const { return raw_; }

        const std::optional<Q>& q() const { return q_; }
        void setQuality(Q quality);

        std::optional<std::string> getParam(const std::string& name) const;
        void setParam(const std::string& name, std::string value);

        std::string toString() const;
        bool isValid() const;

    private:
        Mime::Type top_;
        Mime::Subtype sub_;
        Mime::Suffix suffix_;

        /* Let's save some extra memory allocations by only storing the
     raw MediaType along with indexes of the relevant parts
     Note: experimental for now as it might not be a good idea
  */
        std::string raw_;

        struct Index
        {
            size_t beg;
            size_t end;

            std::string splice(const std::string& str) const
            {
                assert(end >= beg);
                return str.substr(beg, end - beg + 1);
            }
        };

        Index rawSubIndex;
        Index rawSuffixIndex;

        std::unordered_map<std::string, std::string> params;

        std::optional<Q> q_;
    };

    inline bool operator==(const MediaType& lhs, const MediaType& rhs)
    {
        return lhs.top() == rhs.top() && lhs.sub() == rhs.sub() && lhs.suffix() == rhs.suffix();
    }

    inline bool operator!=(const MediaType& lhs, const MediaType& rhs)
    {
        return !operator==(lhs, rhs);
    }

} // namespace Pistache::Http::Mime

#define MIME(top, sub)                                               \
    Pistache::Http::Mime::MediaType(Pistache::Http::Mime::Type::top, \
                                    Pistache::Http::Mime::Subtype::sub)

#define MIME3(top, sub, suffix)                                         \
    Pistache::Http::Mime::MediaType(Pistache::Http::Mime::Type::top,    \
                                    Pistache::Http::Mime::Subtype::sub, \
                                    Pistache::Http::Mime::Suffix::suffix)
