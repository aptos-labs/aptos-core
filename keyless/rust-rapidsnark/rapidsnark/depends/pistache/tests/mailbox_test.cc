/*
 * SPDX-FileCopyrightText: 2018 jcastro
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <gtest/gtest.h>
#include <pistache/mailbox.h>

struct Data
{
    static int num_instances;
    static constexpr int fingerprint = 0xdeadbeef;

    Data()
        : val(Data::fingerprint)
    {
        num_instances++;
    }

    ~Data()
    {
        EXPECT_EQ(val, Data::fingerprint);
        EXPECT_GE(0, --num_instances);
    }

    int val;
};

int Data::num_instances = 0;
constexpr int Data::fingerprint;

TEST(queue_test, destructor_test)
{
    Pistache::Queue<Data> queue;
    EXPECT_TRUE(queue.empty());

    for (int i = 0; i < 5; i++)
    {
        queue.push(Data());
    }
    // Should call Data::~Data 5 times and not 6 (placeholder entry)
}
