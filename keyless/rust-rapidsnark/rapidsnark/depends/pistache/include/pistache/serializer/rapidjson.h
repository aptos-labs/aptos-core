/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 14 mai 2016

   Swagger serializer for RapidJSON
*/

#pragma once

#include <rapidjson/prettywriter.h>

#include <pistache/description.h>
#include <pistache/http_defs.h>
#include <pistache/mime.h>

namespace Pistache::Rest::Serializer
{

    template <typename Writer>
    void serializeInfo(Writer& writer, const Schema::Info& info)
    {
        writer.String("swagger");
        writer.String("2.0");
        writer.String("info");
        writer.StartObject();
        {
            writer.String("title");
            writer.String(info.title.c_str());
            writer.String("version");
            writer.String(info.version.c_str());
            if (!info.description.empty())
            {
                writer.String("description");
                writer.String(info.description.c_str());
            }
            if (!info.termsOfService.empty())
            {
                writer.String("termsOfService");
                writer.String(info.termsOfService.c_str());
            }
        }
        writer.EndObject();
    }

    template <typename Writer>
    void serializePC(Writer& writer, const Schema::ProduceConsume& pc)
    {
        auto serializeMimes = [&](const char* name,
                                  const std::vector<Http::Mime::MediaType>& mimes) {
            if (!mimes.empty())
            {
                writer.String(name);
                writer.StartArray();
                {
                    for (const auto& mime : mimes)
                    {
                        auto str = mime.toString();
                        writer.String(str.c_str());
                    }
                }
                writer.EndArray();
            }
        };

        serializeMimes("consumes", pc.consume);
        serializeMimes("produces", pc.produce);
    }

    template <typename Writer>
    void serializeParameter(Writer& writer, const Schema::Parameter& parameter)
    {
        writer.StartObject();
        {
            writer.String("name");
            writer.String(parameter.name.c_str());
            writer.String("in");
            // @Feature: support other types of parameters
            writer.String("path");
            writer.String("description");
            writer.String(parameter.description.c_str());
            writer.String("required");
            writer.Bool(parameter.required);
            writer.String("type");
            writer.String(parameter.type->typeName());
        }
        writer.EndObject();
    }

    template <typename Writer>
    void serializeResponse(Writer& writer, const Schema::Response& response)
    {
        auto code = std::to_string(static_cast<uint32_t>(response.statusCode));
        writer.String(code.c_str());
        writer.StartObject();
        {
            writer.String("description");
            writer.String(response.description.c_str());
        }
        writer.EndObject();
    }

    template <typename Writer>
    void serializePath(Writer& writer, const Schema::Path& path)
    {
        std::string methodStr(methodString(path.method));
        // So it looks like Swagger requires method to be in lowercase
        std::transform(std::begin(methodStr), std::end(methodStr),
                       std::begin(methodStr), ::tolower);

        writer.String(methodStr.c_str());
        writer.StartObject();
        {
            writer.String("description");
            writer.String(path.description.c_str());
            serializePC(writer, path.pc);

            const auto& parameters = path.parameters;
            if (!parameters.empty())
            {
                writer.String("parameters");
                writer.StartArray();
                {
                    for (const auto& parameter : parameters)
                    {
                        serializeParameter(writer, parameter);
                    }
                }
                writer.EndArray();
            }

            const auto& responses = path.responses;
            if (!responses.empty())
            {
                writer.String("responses");
                writer.StartObject();
                {
                    for (const auto& response : responses)
                    {
                        serializeResponse(writer, response);
                    }
                }
                writer.EndObject();
            }
        }
        writer.EndObject();
    }

    template <typename Writer>
    void serializePathGroups(Writer& writer, const std::string& prefix,
                             const Schema::PathGroup& paths,
                             Schema::PathGroup::Format format)
    {
        writer.String("paths");
        writer.StartObject();
        {
            auto groups = paths.groups();
            for (const auto& group : groups)
            {
                if (group.second.isHidden())
                    continue;

                std::string name(group.first);
                if (!prefix.empty())
                {
                    if (!name.compare(0, prefix.size(), prefix))
                    {
                        name = name.substr(prefix.size());
                    }
                }

                if (format == Schema::PathGroup::Format::Default)
                {
                    writer.String(name.c_str());
                }
                else
                {
                    auto swaggerPath = Schema::Path::swaggerFormat(name);
                    writer.String(swaggerPath.c_str());
                }
                writer.StartObject();
                {
                    for (const auto& path : group.second)
                    {
                        if (!path.hidden)
                            serializePath(writer, path);
                    }
                }
                writer.EndObject();
            }
        }
        writer.EndObject();
    }

    template <typename Writer>
    void serializeDescription(Writer& writer, const Description& desc)
    {
        writer.StartObject();
        {
            serializeInfo(writer, desc.rawInfo());
            auto host         = desc.rawHost();
            auto basePath     = desc.rawBasePath();
            auto schemes      = desc.rawSchemes();
            auto pc           = desc.rawPC();
            const auto& paths = desc.rawPaths();

            if (!host.empty())
            {
                writer.String("host");
                writer.String(host.c_str());
            }
            if (!basePath.empty())
            {
                writer.String("basePath");
                writer.String(basePath.c_str());
            }
            if (!schemes.empty())
            {
                writer.String("schemes");
                writer.StartArray();
                {
                    for (const auto& scheme : schemes)
                    {
                        writer.String(schemeString(scheme));
                    }
                }
                writer.EndArray();
            }
            serializePC(writer, pc);
            serializePathGroups(writer, basePath, paths,
                                Schema::PathGroup::Format::Swagger);
        }
        writer.EndObject();
    }

    inline std::string rapidJson(const Description& desc)
    {
        rapidjson::StringBuffer sb;
        rapidjson::PrettyWriter<rapidjson::StringBuffer> writer(sb);
        serializeDescription(writer, desc);

        return sb.GetString();
    }

} // namespace Pistache::Rest::Serializer
