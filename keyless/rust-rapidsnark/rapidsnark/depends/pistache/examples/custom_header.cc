/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 15 février 2016

   Example of custom headers registering
*/

#include <pistache/http_headers.h>
#include <pistache/net.h>
#include <sys/types.h>

// Quiet a warning about "minor" and "major" being doubly defined.
#ifdef major
#undef major
#endif
#ifdef minor
#undef minor
#endif

using namespace Pistache;

class XProtocolVersion : public Http::Header::Header
{
public:
    NAME("X-Protocol-Version");

    XProtocolVersion() = default;

    XProtocolVersion(uint32_t major, uint32_t minor)
        : maj(major)
        , min(minor)
    { }

    void parse(const std::string& str) override
    {
        auto p = str.find('.');
        std::string major, minor;
        if (p != std::string::npos)
        {
            major = str.substr(0, p);
            minor = str.substr(p + 1);
        }
        else
        {
            major = str;
        }

        maj = std::stoi(major);
        if (!minor.empty())
            min = std::stoi(minor);
    }

    void write(std::ostream& os) const override
    {
        os << maj;
        os << "." << min;
    }

    uint32_t majorVersion() const
    {
        return maj;
    }

    uint32_t minorVersion() const
    {
        return min;
    }

private:
    uint32_t maj = 0;
    uint32_t min = 0;
};

int main()
{
    Http::Header::Registry::instance().registerHeader<XProtocolVersion>();
}
