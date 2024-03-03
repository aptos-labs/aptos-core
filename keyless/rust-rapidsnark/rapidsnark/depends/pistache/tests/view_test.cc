/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>
#include <pistache/view.h>

using namespace Pistache;

template <typename T>
std::vector<T> make_vec(std::initializer_list<T> list)
{
    return std::vector<T>(list);
}

TEST(view_test, test_vector)
{
    auto vec1 = make_vec({ 1, 2, 3, 4 });
    auto v1   = make_view(vec1);
    ASSERT_EQ(v1.size(), 4U);
    ASSERT_EQ(v1[0], 1);
    ASSERT_EQ(v1[3], 4);

    auto v2(v1);
    ASSERT_EQ(v2.size(), 4U);
    ASSERT_EQ(v2[0], 1);
    ASSERT_EQ(v2[3], 4);

    auto vec2 = make_vec({ 2, 4, 6, 8, 10 });
    auto v3   = make_view(vec2, 4);
    ASSERT_EQ(v3.size(), 4U);
    ASSERT_EQ(v3[0], 2);
    ASSERT_EQ(v3[3], 8);
    ASSERT_THROW(v3.at(4), std::invalid_argument);

    ASSERT_EQ(v1, v2);
    ASSERT_NE(v1, v3);

    auto vec3 = make_vec({ 4, 3, 2, 1 });
    auto v4   = make_view(vec3);
    ASSERT_NE(v4, v1);

    std::vector<int> vec4;
    auto v5 = make_view(vec4);
    ASSERT_TRUE(v5.empty());

    auto v6 = make_view(vec3, 0);
    ASSERT_TRUE(v6.empty());
}

TEST(view_test, test_array)
{
    std::array<int, 4> arr1 { { 4, 5, 6, 7 } };
    auto v1 = make_view(arr1);

    ASSERT_EQ(v1.size(), 4U);
    ASSERT_EQ(v1[0], 4);
    ASSERT_EQ(v1[3], 7);

    auto v2 = make_view(arr1, 2);
    ASSERT_EQ(v2.size(), 2U);
    ASSERT_EQ(v2[1], 5);
    ASSERT_THROW(v2.at(3), std::invalid_argument);

    std::array<int, 4> arr2 { { 6, 8, 1, 2 } };
    ASSERT_NE(make_view(arr2), v1);

    std::array<int, 4> arr3 { { 4, 5, 6, 7 } };
    ASSERT_EQ(v1, make_view(arr3));
}

TEST(view_test, string_test)
{
    std::string s1("Hello");
    auto v1 = make_view(s1);

    ASSERT_EQ(v1.size(), 5U);
    ASSERT_EQ(v1[0], 'H');
    ASSERT_EQ(v1[4], 'o');
    ASSERT_EQ(v1, "Hello");

    auto v2 = make_view(s1, 3);
    ASSERT_TRUE(std::strcmp(v2.data(), "Hel"));
    ASSERT_EQ(v2, "Hel");

    std::string s3(s1);
    auto v3 = make_view(s3);
    ASSERT_EQ(v1, v3);
    ASSERT_EQ(v3, s1);

    const char* hello = "Hello";
    ASSERT_EQ(v1, hello);
    ASSERT_EQ(v3, hello);

    std::string s4 = v3;
    ASSERT_EQ(s4, "Hello");
}

TEST(view_test, null_test)
{
    View<int> v1(nullptr);
    ASSERT_TRUE(v1.empty());

    View<int> v2(nullptr, 12);
    ASSERT_TRUE(v2.empty());
}
