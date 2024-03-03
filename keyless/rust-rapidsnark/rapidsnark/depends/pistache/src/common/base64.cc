/*
 * SPDX-FileCopyrightText: 2019 Kip Warner
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// Includes...

// Our headers...
#include <pistache/base64.h>

// Standard C++ / POSIX system headers...
#include <algorithm>
#include <cassert>
#include <cmath>
#include <fstream>
#include <stdexcept>

// Using the standard namespace and Pistache...
using namespace std;

// Calculate length of decoded raw bytes from that would be generated if the
//  base 64 encoded input buffer was decoded. This is not a static method
//  because we need to examine the string...
vector<byte>::size_type Base64Decoder::CalculateDecodedSize() const
{
    // If encoded size was zero, so is decoded size...
    if (m_Base64EncodedString.empty())
        return 0;

    // If non-zero, should always be at least four characters...
    if (m_Base64EncodedString.size() < 4)
        throw runtime_error(
            "Base64 encoded stream should always be at least four bytes.");

    // ...and always a multiple of four bytes because every three decoded bytes
    //  produce four encoded base 64 bytes, which may include padding...
    if ((m_Base64EncodedString.size() % 4) != 0)
        throw runtime_error("Base64 encoded stream length should always be evenly "
                            "divisible by four.");

    // Iterator to walk the encoded string from the beginning...
    auto EndIterator = m_Base64EncodedString.begin();

    // Keep walking along the input buffer trying to decode characters, but
    //  without storing them, until we hit the first character we cannot decode.
    //  This should be the first padding character or end of string...
    while (DecodeCharacter(*EndIterator) < static_cast<byte>(64))
        ++EndIterator;

    // The length of the encoded string is the distance from the beginning to
    //  the first non-decodable character, such as padding...
    const auto InputSize = distance(m_Base64EncodedString.begin(), EndIterator);

    // Calculate decoded size before account for any more decoded bytes within
    //  the trailing padding block...
    const auto DecodedSize = InputSize / 4 * 3;

    // True decoded size depends on how much padding needed to be applied...
    switch (InputSize % 4)
    {
    case 2:
        return DecodedSize + 1;
    case 3:
        return DecodedSize + 2;
    default:
        return DecodedSize;
    }
}

// Decode base 64 encoding into raw bytes...
const vector<byte>& Base64Decoder::Decode()
{
    // Calculate required size of output buffer...
    const auto DecodedSize = CalculateDecodedSize();

    // Allocate sufficient storage...
    m_DecodedData = vector<byte>(DecodedSize, byte(0x00));
    m_DecodedData.shrink_to_fit();

    // Initialize decode input and output iterators...
    string::size_type InputOffset  = 0;
    string::size_type OutputOffset = 0;

    // While there is at least one set of three octets remaining to decode...
    for (string::size_type Index = 2; Index < DecodedSize; Index += 3)
    {
        // Construct octets from sextets...
        m_DecodedData.at(OutputOffset + 0) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 0)) << 2 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 1)) >> 4);
        m_DecodedData.at(OutputOffset + 1) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 1)) << 4 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 2)) >> 2);
        m_DecodedData.at(OutputOffset + 2) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 2)) << 6 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 3)));

        // Reseek i/o pointers...
        InputOffset += 4;
        OutputOffset += 3;
    }

    // There's less than three octets remaining...
    switch (DecodedSize % 3)
    {
    // One octet left to construct...
    case 1:
        m_DecodedData.at(OutputOffset + 0) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 0)) << 2 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 1)) >> 4);
        break;

    // Two octets left to construct...
    case 2:
        m_DecodedData.at(OutputOffset + 0) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 0)) << 2 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 1)) >> 4);
        m_DecodedData.at(OutputOffset + 1) = static_cast<byte>(
            DecodeCharacter(m_Base64EncodedString.at(InputOffset + 1)) << 4 | DecodeCharacter(m_Base64EncodedString.at(InputOffset + 2)) >> 2);
        break;
    }

    // All done. Return constant reference to buffer containing decoded data...
    return m_DecodedData;
}

// Convert an octet character to corresponding sextet, provided it can safely be
//  represented as such. Otherwise return 0xff...
inline byte
Base64Decoder::DecodeCharacter(const unsigned char Character) const
{
    // Capital letter 'A' is ASCII 65 and zero in base 64...
    if ('A' <= Character && Character <= 'Z')
        return static_cast<byte>(Character - 'A');

    // Lowercase letter 'a' is ASCII 97 and 26 in base 64...
    if ('a' <= Character && Character <= 'z')
        return static_cast<byte>(Character - (97 - 26));

    // Numeric digit '0' is ASCII 48 and 52 in base 64...
    if ('0' <= Character && Character <= '9')
        return static_cast<byte>(Character - (48 - 52));

    // '+' is ASCII 43 and 62 in base 64...
    if (Character == '+')
        return static_cast<byte>(62);

    // '/' is ASCII 47 and 63 in base 64...
    if (Character == '/')
        return static_cast<byte>(63);

    // Anything else that's not a 6-bit representation, signal to caller...
    return static_cast<byte>(255);
}

// Calculate length of base 64 string that would need to be generated for raw
//  data of a given length...
string::size_type Base64Encoder::CalculateEncodedSize(
    const vector<byte>::size_type DecodedSize) noexcept
{
    // First term calcualtes the unpadded length. The bitwise and rounds up to
    //  the nearest multiple of four to add padding...
    return ((4 * DecodedSize / 3) + 3) & ~3;
}

// Encode raw data input buffer to base 64...
const string& Base64Encoder::Encode() noexcept
{
    // Allocate precise storage for the output buffer...
    m_Base64EncodedString = string(CalculateEncodedSize(m_InputBuffer.size()), '!');
    m_Base64EncodedString.shrink_to_fit();

    // Number of complete octet triplets...
    const auto OctetTriplets = m_InputBuffer.size() / 3;

    // Initialize encode input and output offset registers...
    string::size_type InputOffset  = 0;
    string::size_type OutputOffset = 0;

    // While there are still complete octet triplets remaining...
    for (string::size_type Index = 0; Index < OctetTriplets; ++Index)
    {
        // Encode first sextet from first octet...
        m_Base64EncodedString.at(OutputOffset + 0) = EncodeByte(static_cast<byte>(m_InputBuffer.at(InputOffset + 0) >> 2));

        // Encode second sextet from first and second octet....
        m_Base64EncodedString.at(OutputOffset + 1) = EncodeByte(static_cast<byte>(
            (m_InputBuffer.at(InputOffset + 0) & static_cast<byte>(0x03)) << 4 | m_InputBuffer.at(InputOffset + 1) >> 4));

        // Encode third sextet from second and third octet...
        m_Base64EncodedString.at(OutputOffset + 2) = EncodeByte(static_cast<byte>(
            (m_InputBuffer.at(InputOffset + 1) & static_cast<byte>(0x0F)) << 2 | m_InputBuffer.at(InputOffset + 2) >> 6));

        // Encode fourth sextet from third octet...
        m_Base64EncodedString.at(OutputOffset + 3) = EncodeByte(m_InputBuffer.at(InputOffset + 2) & static_cast<byte>(0x3F));

        // Stride i/o pointers...
        InputOffset += 3;
        OutputOffset += 4;
    }

    // Since the length of padded base 64 encoding must always be a multiple of
    //  four, after the last octet triplet, were there any additional octets in
    //  the input to encode that were less than three in number?
    switch (m_InputBuffer.size() % 3)
    {
    // Exactly one trailing octet followed...
    case 1:

        // Encode first sextet from remaining octet...
        m_Base64EncodedString.at(OutputOffset + 0) = EncodeByte(static_cast<byte>(m_InputBuffer.at(InputOffset + 0) >> 2));

        // Encode second sextet from remaining octet and empty second one...
        m_Base64EncodedString.at(OutputOffset + 1) = EncodeByte(
            (m_InputBuffer.at(InputOffset + 0) & static_cast<byte>(0x03)) << 4);

        // Padd the two sextets with two padding characters to ensure the
        //  total length is a multiple of four...
        m_Base64EncodedString.at(OutputOffset + 2) = '=';
        m_Base64EncodedString.at(OutputOffset + 3) = '=';
        break;

    // Exactly two trailing octets followed...
    case 2:

        // Encode first sextet from first octet...
        m_Base64EncodedString.at(OutputOffset + 0) = EncodeByte(static_cast<byte>(m_InputBuffer.at(InputOffset + 0) >> 2));

        // Encode second sextet from first and second octet...
        m_Base64EncodedString.at(OutputOffset + 1) = EncodeByte(static_cast<byte>(
            (m_InputBuffer.at(InputOffset + 0) & static_cast<byte>(0x03)) << 4 | m_InputBuffer.at(InputOffset + 1) >> 4));

        // Encode third sextet from second and dummy third octet...
        m_Base64EncodedString.at(OutputOffset + 2) = EncodeByte(
            (m_InputBuffer.at(InputOffset + 1) & static_cast<byte>(0x0F)) << 2);

        // Padd three sextets with a single padding character to ensure the
        //  total length is a multiple of four...
        m_Base64EncodedString.at(OutputOffset + 3) = '=';
        break;
    }

    // Return constant reference to encoded data to caller...
    return m_Base64EncodedString;
}

// Encode single binary byte to 6-bit base 64 character...
inline unsigned char Base64Encoder::EncodeByte(const byte Byte) const
{
    // Capital letter 'A' is ASCII 65 and zero in base 64...
    auto ch = static_cast<unsigned char>(Byte);
    if (ch < 26)
        return static_cast<unsigned char>(ch + 'A');

    // Lowercase letter 'a' is ASCII 97 and 26 in base 64...
    if (ch < 52)
        return static_cast<unsigned char>(ch + 71);

    // Numeric digit '0' is ASCII 48 and 52 in base 64...
    if (ch < 62)
        return static_cast<unsigned char>(ch - 4);

    // '+' is ASCII 43 and 62 in base 64...
    if (ch == 62)
        return '+';

    // '/' is ASCII 47 and 63 in base 64...
    if (ch == 63)
        return '/';

    // And lastly anything that can't be represented in 6-bits we return 64...
    return 64;
}

// Encode a string into base 64 format...
string Base64Encoder::EncodeString(const string& StringInput)
{
    // Allocate storage for binary form of message...
    vector<byte> BinaryInput(StringInput.size());

    // Convert message to binary form...
    transform(StringInput.begin(), StringInput.end(), BinaryInput.begin(),
              [](const char Character) { return byte(Character); });

    // Encode to base 64...
    Base64Encoder Encoder(BinaryInput);

    // Return encoded string to caller by value...
    return Encoder.Encode();
}
