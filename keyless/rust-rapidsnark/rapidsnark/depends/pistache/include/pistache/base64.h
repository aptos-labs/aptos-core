/*
 * SPDX-FileCopyrightText: 2019 Kip Warner
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// Multiple include protection...
#ifndef _BASE_64_H_
#define _BASE_64_H_

// Includes...

// Build environment configuration...
#include <pistache/config.h>

// Standard C++ / POSIX system headers...
#include <cstddef>
#include <string>
#include <vector>

#if __cplusplus < 201703L
namespace std
{
    typedef uint8_t byte;
}
#endif

// A class for performing decoding to raw bytes from base 64 encoding...
class Base64Decoder
{
    // Public methods...
public:
    // Constructor...
    explicit Base64Decoder(const std::string& Base64EncodedString)
        : m_Base64EncodedString(Base64EncodedString)
    { }

    // Calculate length of decoded raw bytes from that would be generated if
    //  the base 64 encoded input buffer was decoded. This is not a static
    //  method because we need to examine the string...
    std::vector<std::byte>::size_type CalculateDecodedSize() const;

    // Decode base 64 encoding into raw bytes...
    const std::vector<std::byte>& Decode();

    // Get raw decoded data...
    const std::vector<std::byte>& GetRawDecodedData() const noexcept
    {
        return m_DecodedData;
    }

    // Protected methods...
protected:
    // Convert an octet character to corresponding sextet, provided it can
    //  safely be represented as such. Otherwise return 0xff...
    std::byte DecodeCharacter(const unsigned char Character) const;

    // Protected attributes...
protected:
    // Base 64 encoded string to decode...
    const std::string& m_Base64EncodedString;

    // Decoded raw data...
    std::vector<std::byte> m_DecodedData;
};

// A class for performing base 64 encoding from raw bytes...
class Base64Encoder
{
    // Public methods...
public:
    // Construct encoder to encode from a raw input buffer...
    explicit Base64Encoder(const std::vector<std::byte>& InputBuffer)
        : m_InputBuffer(InputBuffer)
    { }

    // Calculate length of base 64 string that would need to be generated
    //  for raw data of a given length...
    static std::string::size_type CalculateEncodedSize(
        const std::vector<std::byte>::size_type DecodedSize) noexcept;

    // Encode raw data input buffer to base 64...
    const std::string& Encode() noexcept;

    // Encode a string into base 64 format...
    static std::string EncodeString(const std::string& StringInput);

    // Get the encoded data...
    const std::string& GetBase64EncodedString() const noexcept
    {
        return m_Base64EncodedString;
    }

    // Protected methods...
protected:
    // Encode single binary byte to 6-bit base 64 character...
    unsigned char EncodeByte(const std::byte Byte) const;

    // Protected attributes...
protected:
    // Raw bytes to encode to base 64 string...
    const std::vector<std::byte>& m_InputBuffer;

    // Base64 encoded string...
    std::string m_Base64EncodedString;
};

// Multiple include protection...
#endif
