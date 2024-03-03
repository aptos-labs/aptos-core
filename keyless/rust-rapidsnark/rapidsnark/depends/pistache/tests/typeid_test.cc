/*
 * SPDX-FileCopyrightText: 2015 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>
#include <pistache/typeid.h>

using namespace Pistache;

TEST(type_id_test, basic_test)
{
    ASSERT_EQ(TypeId::of<int>(), TypeId::of<int>());

    ASSERT_NE(TypeId::of<int>(), TypeId::of<int*>());
    ASSERT_NE(TypeId::of<int>(), TypeId::of<int&>());
    ASSERT_NE(TypeId::of<int&>(), TypeId::of<int&&>());

    ASSERT_EQ(TypeId::of<int&>(), TypeId::of<int&>());

    ASSERT_NE(TypeId::of<int&>(), TypeId::of<const int&>());
    ASSERT_NE(TypeId::of<int* const>(), TypeId::of<int const*>());
}
