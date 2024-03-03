/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* stream.cc
   Mathieu Stefani, 05 September 2015

*/

#include <pistache/stream.h>

#include <algorithm>
#include <cassert>
#include <iostream>
#include <string>

#include <fcntl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

namespace Pistache
{

    RawBuffer::RawBuffer(std::string data, size_t length)
        : data_(std::move(data))
        , length_(length)
    { }

    RawBuffer::RawBuffer(const char* data, size_t length)
        : data_()
        , length_(length)
    {
        // input may come not from a ZTS - copy only length_ characters.
        data_.assign(data, length_);
    }

    RawBuffer RawBuffer::copy(size_t fromIndex) const
    {
        if (data_.empty())
            return RawBuffer();

        if (length_ < fromIndex)
            throw std::range_error(
                "Trying to detach buffer from an index bigger than lengthght.");

        auto newDatalength  = length_ - fromIndex;
        std::string newData = data_.substr(fromIndex, newDatalength);

        return RawBuffer(std::move(newData), newDatalength);
    }

    const std::string& RawBuffer::data() const { return data_; }

    size_t RawBuffer::size() const { return length_; }

    FileBuffer::FileBuffer(const std::string& fileName)
        : fileName_(fileName)
        , fd_(-1)
        , size_(0)
    {
        if (fileName.empty())
        {
            throw std::runtime_error("Empty fileName");
        }

        int fd = open(fileName.c_str(), O_RDONLY);
        if (fd == -1)
        {
            throw std::runtime_error("Could not open file");
        }

        struct stat sb;
        int res = ::fstat(fd, &sb);
        if (res == -1)
        {
            close(fd);
            throw std::runtime_error("Could not get file stats");
        }

        fd_   = fd;
        size_ = sb.st_size;
    }

    Fd FileBuffer::fd() const { return fd_; }

    size_t FileBuffer::size() const { return size_; }

    DynamicStreamBuf::DynamicStreamBuf(size_t size, size_t maxSize)
        : data_()
        , maxSize_(maxSize)
    {
        assert(size <= maxSize);

        reserve(size);
    }

    DynamicStreamBuf::DynamicStreamBuf(DynamicStreamBuf&& other)
        : data_(std::move(other.data_))
        , maxSize_(other.maxSize_)
    {
        setp(other.pptr(), other.epptr());
        other.setp(nullptr, nullptr);
    }

    DynamicStreamBuf& DynamicStreamBuf::operator=(DynamicStreamBuf&& other)
    {
        if (&other != this)
        {
            data_    = std::move(other.data_);
            maxSize_ = other.maxSize_;
            setp(other.pptr(), other.epptr());
            other.setp(nullptr, nullptr);
        }

        return *this;
    }

    RawBuffer DynamicStreamBuf::buffer() const
    {
        return RawBuffer(data_.data(), pptr() - data_.data());
    }

    size_t DynamicStreamBuf::maxSize() const { return maxSize_; }

    void DynamicStreamBuf::clear()
    {
        // reset stream buffer to the whole backing storage.
        this->setp(data_.data(), data_.data() + data_.size());
    }

    DynamicStreamBuf::int_type
    DynamicStreamBuf::overflow(DynamicStreamBuf::int_type ch)
    {
        if (!traits_type::eq_int_type(ch, traits_type::eof()))
        {
            const auto size = data_.size();
            if (size < maxSize_)
            {
                reserve((size ? size : 1u) * 2);
                *pptr() = static_cast<char>(ch);
                pbump(1);
                return traits_type::not_eof(ch);
            }
        }

        return traits_type::eof();
    }

    void DynamicStreamBuf::reserve(size_t size)
    {
        if (size > maxSize_)
        {
            size = maxSize_;
        }

        const size_t oldSize = data_.size();
        data_.resize(size);
        this->setp(data_.data() + oldSize, data_.data() + size);
    }

    bool StreamCursor::advance(size_t count)
    {
        if (static_cast<ssize_t>(count) > buf->in_avail())
            return false;

        for (size_t i = 0; i < count; ++i)
        {
            buf->sbumpc();
        }

        return true;
    }

    bool StreamCursor::eol() const { return buf->sgetc() == CR && next() == LF; }

    bool StreamCursor::eof() const { return remaining() == 0; }

    int StreamCursor::next() const
    {
        if (buf->in_avail() < 1)
            return Eof;

        return buf->snext();
    }

    char StreamCursor::current() const { return static_cast<char>(buf->sgetc()); }

    const char* StreamCursor::offset() const { return buf->curptr(); }

    const char* StreamCursor::offset(size_t off) const
    {
        return buf->begptr() + off;
    }

    size_t StreamCursor::diff(size_t other) const
    {
        return buf->position() - other;
    }

    size_t StreamCursor::diff(const StreamCursor& other) const
    {
        return other.buf->position() - buf->position();
    }

    size_t StreamCursor::remaining() const { return buf->in_avail(); }

    void StreamCursor::reset() { buf->reset(); }

    bool match_raw(const void* buf, size_t len, StreamCursor& cursor)
    {
        if (cursor.remaining() < len)
            return false;

        if (memcmp(cursor.offset(), buf, len) == 0)
        {
            cursor.advance(len);
            return true;
        }

        return false;
    }

    bool match_string(const char* str, size_t len, StreamCursor& cursor,
                      CaseSensitivity cs)
    {
        if (cursor.remaining() < len)
            return false;

        if (cs == CaseSensitivity::Sensitive)
        {
            if (strncmp(cursor.offset(), str, len) == 0)
            {
                cursor.advance(len);
                return true;
            }
        }
        else
        {
            const char* off = cursor.offset();
            for (size_t i = 0; i < len; ++i)
            {
                const char lhs = static_cast<char>(std::tolower(str[i]));
                const char rhs = static_cast<char>(std::tolower(off[i]));
                if (lhs != rhs)
                    return false;
            }

            cursor.advance(len);
            return true;
        }

        return false;
    }

    bool match_literal(char c, StreamCursor& cursor, CaseSensitivity cs)
    {
        if (cursor.eof())
            return false;

        char lhs = (cs == CaseSensitivity::Sensitive ? c : static_cast<char>(std::tolower(c)));
        char rhs = (cs == CaseSensitivity::Sensitive
                        ? cursor.current()
                        : static_cast<char>(std::tolower(cursor.current())));

        if (lhs == rhs)
        {
            cursor.advance(1);
            return true;
        }

        return false;
    }

    bool match_until(char c, StreamCursor& cursor, CaseSensitivity cs)
    {
        return match_until({ c }, cursor, cs);
    }

    bool match_until(std::initializer_list<char> chars, StreamCursor& cursor,
                     CaseSensitivity cs)
    {
        if (cursor.eof())
            return false;

        auto find = [&](char val) {
            for (auto c : chars)
            {
                char lhs = cs == CaseSensitivity::Sensitive
                    ? c
                    : static_cast<char>(std::tolower(c));
                char rhs = cs == CaseSensitivity::Insensitive
                    ? val
                    : static_cast<char>(std::tolower(val));

                if (lhs == rhs)
                    return true;
            }

            return false;
        };

        while (!cursor.eof())
        {
            const char c = cursor.current();
            if (find(c))
                return true;
            cursor.advance(1);
        }

        return false;
    }

    bool match_double(double* val, StreamCursor& cursor)
    {
        // @Todo: strtod does not support a length argument
        char* end;
        *val = strtod(cursor.offset(), &end);
        if (end == cursor.offset())
            return false;

        cursor.advance(static_cast<ptrdiff_t>(end - cursor.offset()));
        return true;
    }

    void skip_whitespaces(StreamCursor& cursor)
    {
        if (cursor.eof())
            return;

        int c;
        while ((c = cursor.current()) != StreamCursor::Eof && (c == ' ' || c == '\t'))
        {
            cursor.advance(1);
        }
    }

} // namespace Pistache
