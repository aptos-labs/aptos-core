/*
 * SPDX-FileCopyrightText: 2019 knowledge4igor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <pistache/stream.h>

#include <gtest/gtest.h>

#include <climits>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <string>

using namespace Pistache;

TEST(stream, test_buffer)
{
    const char str[] = "test_string";
    const size_t len = strlen(str);
    RawBuffer buffer1(str, len);

    ASSERT_THROW(buffer1.copy(2 * len), std::range_error);

    RawBuffer buffer2 = buffer1.copy();
    ASSERT_EQ(buffer2.size(), len);
    ASSERT_EQ(buffer2.data(), "test_string");

    RawBuffer buffer3;
    ASSERT_EQ(buffer3.size(), 0u);

    RawBuffer buffer4 = buffer3.copy();
    ASSERT_EQ(buffer4.size(), 0u);

    RawBuffer buffer5 = buffer1.copy(5u);
    ASSERT_EQ(buffer5.size(), 6u);
    ASSERT_EQ(buffer5.data(), "string");
}

TEST(stream, test_file_buffer)
{
    char fileName[PATH_MAX] = "/tmp/pistacheioXXXXXX";
    if (!mkstemp(fileName))
    {
        std::cerr << "No suitable filename can be generated!" << std::endl;
    }
    std::cout << "Temporary file name: " << fileName << std::endl;

    const std::string dataToWrite("Hello World!");
    std::ofstream tmpFile;
    tmpFile.open(fileName);
    tmpFile << dataToWrite;
    tmpFile.close();

    FileBuffer fileBuffer(fileName);

    ASSERT_NE(fileBuffer.fd(), -1);
    ASSERT_EQ(fileBuffer.size(), dataToWrite.size());

    std::remove(fileName);
}

TEST(stream, test_dyn_buffer)
{
    DynamicStreamBuf buf(128, Const::MaxBuffer);
    ASSERT_EQ(buf.maxSize(), Const::MaxBuffer);

    {
        std::ostream os(&buf);

        for (unsigned i = 0; i < 128; ++i)
        {
            os << "A";
        }
    }

    auto rawbuf = buf.buffer();

    ASSERT_EQ(rawbuf.size(), 128u);
    ASSERT_EQ(rawbuf.data().size(), 128u);
    ASSERT_EQ(strlen(rawbuf.data().c_str()), 128u);
}

TEST(stream, test_array_buffer)
{
    ArrayStreamBuf<char> buffer(4);

    const char* part1 = "abcd";
    ASSERT_TRUE(buffer.feed(part1, strlen(part1)));

    const char* part2 = "efgh";
    ASSERT_FALSE(buffer.feed(part2, strlen(part2)));
}

TEST(stream, test_cursor_advance_for_array)
{
    ArrayStreamBuf<char> buffer(Const::MaxBuffer);
    StreamCursor cursor { &buffer };

    const char* part1 = "abcd";
    buffer.feed(part1, strlen(part1));

    ASSERT_EQ(cursor.current(), 'a');

    ASSERT_TRUE(cursor.advance(1));
    ASSERT_EQ(cursor.current(), 'b');

    ASSERT_TRUE(cursor.advance(0));
    ASSERT_EQ(cursor.current(), 'b');

    ASSERT_TRUE(cursor.advance(1));
    ASSERT_EQ(cursor.current(), 'c');

    const char* part2 = "efgh";
    buffer.feed(part2, strlen(part2));

    ASSERT_TRUE(cursor.advance(2));
    ASSERT_EQ(cursor.current(), 'e');

    ASSERT_FALSE(cursor.advance(5));
}

TEST(stream, test_cursor_remaining_for_array)
{
    ArrayStreamBuf<char> buffer(Const::MaxBuffer);
    StreamCursor cursor { &buffer };

    const char* data = "abcd";
    ASSERT_TRUE(buffer.feed(data, strlen(data)));
    ASSERT_EQ(cursor.remaining(), 4u);

    cursor.advance(2);
    ASSERT_EQ(cursor.remaining(), 2u);

    cursor.advance(1);
    ASSERT_EQ(cursor.remaining(), 1u);

    cursor.advance(1);
    ASSERT_EQ(cursor.remaining(), 0u);
}

TEST(stream, test_cursor_eol_eof_for_array)
{
    ArrayStreamBuf<char> buffer(Const::MaxBuffer);
    StreamCursor cursor { &buffer };

    const char* data = "abcd\r\nefgh";
    ASSERT_TRUE(buffer.feed(data, strlen(data)));

    cursor.advance(4);
    ASSERT_TRUE(cursor.eol());
    ASSERT_FALSE(cursor.eof());

    cursor.advance(2);
    ASSERT_FALSE(cursor.eol());
    ASSERT_FALSE(cursor.eof());

    cursor.advance(4);
    ASSERT_FALSE(cursor.eol());
    ASSERT_TRUE(cursor.eof());
}

TEST(stream, test_cursor_offset_for_array)
{
    ArrayStreamBuf<char> buffer(Const::MaxBuffer);
    StreamCursor cursor { &buffer };

    const char* data = "abcdefgh";
    ASSERT_TRUE(buffer.feed(data, strlen(data)));

    const size_t shift = 4u;
    cursor.advance(shift);

    std::string result { cursor.offset(), strlen(data) - shift };
    ASSERT_EQ(result, "efgh");
}

TEST(stream, test_cursor_diff_for_array)
{
    ArrayStreamBuf<char> buffer1(Const::MaxBuffer);
    StreamCursor first_cursor { &buffer1 };
    ArrayStreamBuf<char> buffer2(Const::MaxBuffer);
    StreamCursor second_cursor { &buffer2 };

    const char* data = "abcdefgh";
    ASSERT_TRUE(buffer1.feed(data, strlen(data)));
    ASSERT_TRUE(buffer2.feed(data, strlen(data)));

    ASSERT_EQ(first_cursor.diff(second_cursor), 0u);
    ASSERT_EQ(second_cursor.diff(first_cursor), 0u);

    first_cursor.advance(4);
    ASSERT_EQ(second_cursor.diff(first_cursor), 4u);

    second_cursor.advance(4);
    ASSERT_EQ(second_cursor.diff(first_cursor), 0u);
}
