/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* flags.h
   Mathieu Stefani, 18 August 2015

   Make it easy to have bitwise operators for scoped or unscoped enumerations
*/

#pragma once

#include <climits>
#include <iostream>
#include <type_traits>

namespace Pistache
{

    // Looks like gcc 4.6 does not implement std::underlying_type
    namespace detail
    {
        template <size_t N>
        struct TypeStorage;

        template <>
        struct TypeStorage<sizeof(uint8_t)>
        {
            typedef uint8_t Type;
        };
        template <>
        struct TypeStorage<sizeof(uint16_t)>
        {
            typedef uint16_t Type;
        };
        template <>
        struct TypeStorage<sizeof(uint32_t)>
        {
            typedef uint32_t Type;
        };
        template <>
        struct TypeStorage<sizeof(uint64_t)>
        {
            typedef uint64_t Type;
        };

        template <typename T>
        struct UnderlyingType
        {
            typedef typename TypeStorage<sizeof(T)>::Type Type;
        };

        template <typename Enum>
        struct HasNone
        {
            template <typename U>
            static auto test(U*) -> decltype(U::None, std::true_type());

            template <typename U>
            static auto test(...) -> std::false_type;

            static constexpr bool value = std::is_same<decltype(test<Enum>(0)), std::true_type>::value;
        };
    } // namespace detail

    template <typename T>
    class Flags
    {
    public:
        typedef typename detail::UnderlyingType<T>::Type Type;

        static_assert(std::is_enum<T>::value, "Flags only works with enumerations");
        static_assert(detail::HasNone<T>::value, "The enumartion needs a None value");
        static_assert(static_cast<Type>(T::None) == 0, "None should be 0");

        Flags()
            : val(T::None)
        { }

        explicit Flags(T _val)
            : val(_val)
        { }

#define DEFINE_BITWISE_OP_CONST(Op)                                                \
    Flags<T> operator Op(T rhs) const                                              \
    {                                                                              \
        return Flags<T>(                                                           \
            static_cast<T>(static_cast<Type>(val) Op static_cast<Type>(rhs)));     \
    }                                                                              \
                                                                                   \
    Flags<T> operator Op(Flags<T> rhs) const                                       \
    {                                                                              \
        return Flags<T>(                                                           \
            static_cast<T>(static_cast<Type>(val) Op static_cast<Type>(rhs.val))); \
    }

        DEFINE_BITWISE_OP_CONST(|)
        DEFINE_BITWISE_OP_CONST(&)
        DEFINE_BITWISE_OP_CONST(^)

#undef DEFINE_BITWISE_OP_CONST

#define DEFINE_BITWISE_OP(Op)                                                       \
    Flags<T>& operator Op##=(T rhs)                                                 \
    {                                                                               \
        val = static_cast<T>(static_cast<Type>(val) Op static_cast<Type>(rhs));     \
        return *this;                                                               \
    }                                                                               \
                                                                                    \
    Flags<T>& operator Op##=(Flags<T> rhs)                                          \
    {                                                                               \
        val = static_cast<T>(static_cast<Type>(val) Op static_cast<Type>(rhs.val)); \
        return *this;                                                               \
    }

        DEFINE_BITWISE_OP(|)
        DEFINE_BITWISE_OP(&)
        DEFINE_BITWISE_OP(^)

#undef DEFINE_BITWISE_OP

        bool hasFlag(T flag) const
        {
            return static_cast<Type>(val) & static_cast<Type>(flag);
        }

        Flags<T>& setFlag(T flag)
        {
            *this |= flag;
            return *this;
        }

        Flags<T>& toggleFlag(T flag) { return *this ^= flag; }

        operator T() const { return val; }

    private:
        T val;
    };

} // namespace Pistache

#define DEFINE_BITWISE_OP(Op, T)                                          \
    inline T operator Op(T lhs, T rhs)                                    \
    {                                                                     \
        typedef Pistache::detail::UnderlyingType<T>::Type UnderlyingType; \
        return static_cast<T>(static_cast<UnderlyingType>(lhs)            \
                                  Op static_cast<UnderlyingType>(rhs));   \
    }

#define DECLARE_FLAGS_OPERATORS(T) \
    DEFINE_BITWISE_OP(&, T)        \
    DEFINE_BITWISE_OP(|, T)

template <typename T>
std::ostream& operator<<(std::ostream& os, Pistache::Flags<T> flags)
{
    typedef typename Pistache::detail::UnderlyingType<T>::Type UnderlyingType;

    auto val = static_cast<UnderlyingType>(static_cast<T>(flags));
    for (ssize_t i = (sizeof(UnderlyingType) * CHAR_BIT) - 1; i >= 0; --i)
    {
        os << ((val >> i) & 0x1);
    }

    return os;
}
