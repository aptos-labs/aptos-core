/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 28 janvier 2016

   Simple Prototype design pattern implement
*/

#pragma once

#include <memory>
#include <type_traits>

namespace Pistache
{

    /* In a sense, a Prototype is just a class that provides a clone() method */
    template <typename Class>
    struct Prototype
    {
    public:
        virtual ~Prototype()                         = default;
        virtual std::shared_ptr<Class> clone() const = 0;
    };

} // namespace Pistache

#define PROTOTYPE_OF(Base, Class)                \
public:                                          \
    std::shared_ptr<Base> clone() const override \
    {                                            \
        return std::make_shared<Class>(*this);   \
    }
