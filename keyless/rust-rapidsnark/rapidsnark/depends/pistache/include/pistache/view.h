/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* view.h
   Mathieu Stefani, 19 janvier 2016

   A non-owning range of contiguous memory that is represented by
   a pointer to the beginning of the memory and the size.
*/

#pragma once

#include <array>
#include <cstring>
#include <stdexcept>
#include <string>
#include <vector>

namespace Pistache
{

    template <typename T>
    class ViewBase
    {
    public:
        typedef T value_type;
        typedef T& reference;
        typedef T* pointer;
        typedef const T* const_pointer;
        typedef const T& const_reference;

        typedef const_pointer iterator;
        typedef const_pointer const_iterator;

        explicit ViewBase(std::nullptr_t)
            : begin_(nullptr)
            , size_(0)
        { }

        explicit ViewBase(std::nullptr_t, size_t)
            : begin_(nullptr)
            , size_(0)
        { }

        explicit ViewBase(const T* begin, size_t size)
            : begin_(begin)
            , size_(size)
        { }

        explicit ViewBase(const T* begin, const T* end)
            : begin_(begin)
            , size_(0)
        {
            if (begin > end)
            {
                throw std::invalid_argument("begin > end");
            }

            size_ = std::distance(begin, end);
        }

        size_t size() const { return size_; }

        const T& operator[](size_t index) { return begin_[index]; }

        const T& operator[](size_t index) const { return begin_[index]; }

        const T& at(size_t index)
        {
            if (index >= size_)
                throw std::invalid_argument("index > size");

            return begin_[index];
        }

        const T& at(size_t index) const
        {
            if (index >= size_)
                throw std::invalid_argument("index > size");

            return begin_[index];
        }

        const_iterator begin() const { return const_iterator(begin_); }

        const_iterator end() const { return const_iterator(begin_ + size_); }

        bool operator==(const ViewBase<T>& other) const
        {
            return size() == other.size() && std::equal(begin(), end(), other.begin());
        }

        bool operator!=(const ViewBase<T>& other) const { return !(*this == other); }

        bool empty() const { return begin_ == nullptr || size_ == 0; }

        const T* data() const { return begin_; }

    private:
        const T* begin_;
        size_t size_;
    };

    template <typename T>
    struct View : public ViewBase<T>
    {
        explicit View(std::nullptr_t)
            : ViewBase<T>(nullptr)
        { }

        explicit View(std::nullptr_t, size_t)
            : ViewBase<T>(nullptr, 0)
        { }

        explicit View(const T* begin, size_t size)
            : ViewBase<T>(begin, size)
        { }

        explicit View(const T* begin, const T* end)
            : ViewBase<T>(begin, end)
        { }
    };

    template <>
    struct View<std::string> : public ViewBase<char>
    {

        typedef ViewBase<char> Base;

        explicit View(std::nullptr_t)
            : Base(nullptr)
        { }

        explicit View(const char* begin, size_t end)
            : Base(begin, end)
        { }

        explicit View(const char* begin, const char* end)
            : Base(begin, end)
        { }

        using Base::operator==;

        bool operator==(const char* str) const
        {
            return equals(str, std::strlen(str));
        }

        bool operator==(const std::string& str) const
        {
            return equals(str.c_str(), str.size());
        }

        template <size_t N>
        bool operator==(const char (&arr)[N]) const
        {
            return equals(arr, N);
        }

        std::string toString() const { return std::string(data(), size()); }

        operator std::string() const { return toString(); }

    private:
        bool equals(const char* str, size_t length) const
        {
            if (length != size())
                return false;

            return std::equal(begin(), end(), str);
        }
    };

    typedef View<std::string> StringView;

    namespace impl
    {
        template <typename T>
        struct ViewBuilder;

        template <typename T, size_t N>
        struct ViewBuilder<std::array<T, N>>
        {
            typedef T Type;

            static View<T> build(const std::array<T, N>& arr)
            {
                return buildSized(arr, arr.size());
            }

            static View<T> buildSized(const std::array<T, N>& arr, size_t size)
            {
                if (size > arr.size())
                    throw std::invalid_argument("out of bounds size");

                return View<T>(arr.data(), size);
            }
        };

        template <typename T, size_t N>
        struct ViewBuilder<T (&)[N]>
        {
            typedef T Type;

            static View<T> build(T (&arr)[N]) { return buildSize(arr, N); }

            static View<T> buildSize(T (&arr)[N], size_t size)
            {
                if (size > N)
                    throw std::invalid_argument("out of bounds size");

                return View<T>(&arr[0], size);
            }
        };

        template <typename T>
        struct ViewBuilder<std::vector<T>>
        {
            typedef T Type;

            static View<T> build(const std::vector<T>& vec)
            {
                return buildSized(vec, vec.size());
            }

            static View<T> buildSized(const std::vector<T>& vec, size_t size)
            {
                if (size > vec.size())
                    throw std::invalid_argument("out of bounds size");

                return View<T>(vec.data(), size);
            }
        };

        template <>
        struct ViewBuilder<std::string>
        {
            typedef std::string Type;

            static StringView build(const std::string& str)
            {
                return buildSized(str, str.size());
            }

            static StringView buildSized(const std::string& str, size_t size)
            {
                if (size > str.size())
                    throw std::invalid_argument("out of bounds size");

                return StringView(str.data(), size);
            }
        };
    } // namespace impl

    template <typename Cont>
    View<typename impl::ViewBuilder<Cont>::Type> make_view(const Cont& cont)
    {
        return impl::ViewBuilder<Cont>::build(cont);
    }

    template <typename Cont>
    View<typename impl::ViewBuilder<Cont>::Type> make_view(const Cont& cont,
                                                           size_t size)
    {
        return impl::ViewBuilder<Cont>::buildSized(cont, size);
    }

} // namespace Pistache
