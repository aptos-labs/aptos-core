/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 28 février 2016

   A collection of sample iterator adapters
*/

#pragma once

namespace Pistache
{

    template <typename Map>
    struct FlatMapIteratorAdapter
    {
        typedef typename Map::key_type Key;
        typedef typename Map::mapped_type Value;
        typedef typename Map::const_iterator const_iterator;

        explicit FlatMapIteratorAdapter(const_iterator _it)
            : it(_it)
        { }

        FlatMapIteratorAdapter& operator++()
        {
            ++it;
            return *this;
        }

        const Value& operator*() { return it->second; }

        bool operator==(FlatMapIteratorAdapter other) { return other.it == it; }

        bool operator!=(FlatMapIteratorAdapter other) { return !(*this == other); }

    private:
        const_iterator it;
    };

    template <typename Map>
    FlatMapIteratorAdapter<Map>
    makeFlatMapIterator(const Map&, typename Map::const_iterator it)
    {
        return FlatMapIteratorAdapter<Map>(it);
    }

} // namespace Pistache
