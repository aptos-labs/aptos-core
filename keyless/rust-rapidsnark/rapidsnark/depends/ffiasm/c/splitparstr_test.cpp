#include "gtest/gtest.h"
#include "splitparstr.hpp"

namespace {

TEST(splitParStr, SplitIn2) {
  auto v = splitParStr("123,456");

  ASSERT_EQ(v.size(), 2);
  ASSERT_STREQ(v[0].c_str(), "123");
  ASSERT_STREQ(v[1].c_str(), "456");
}

TEST(splitParStr, SplitIn3) {
  auto v = splitParStr("123,456,789");

  ASSERT_EQ(v.size(), 3);
  ASSERT_STREQ(v[0].c_str(), "123");
  ASSERT_STREQ(v[1].c_str(), "456");
  ASSERT_STREQ(v[2].c_str(), "789");
}

TEST(splitParStr, SplitIn2InParenthesis) {
  auto v = splitParStr("(123,456)");

  ASSERT_EQ(v.size(), 2);
  ASSERT_STREQ(v[0].c_str(), "123");
  ASSERT_STREQ(v[1].c_str(), "456");
}

TEST(splitParStr, SplitIn2InManyParenthesis) {
  auto v = splitParStr("(((123,456),(789,abc)))");
/*
  for (int i=0; i<v.size(); i++) {
    std::cout << v[i] << std::endl;
  }
*/
  ASSERT_EQ(v.size(), 2);
  ASSERT_STREQ(v[0].c_str(), "123,456");
  ASSERT_STREQ(v[1].c_str(), "789,abc");
}

TEST(splitParStr, SplitAndPadd) {
  auto v = splitParStr(" ( (), ((123) , 456)  , (789 , abc) )  ");
/*
  for (int i=0; i<v.size(); i++) {
    std::cout << v[i] << std::endl;
  }
*/
  ASSERT_EQ(v.size(), 3);
  ASSERT_STREQ(v[0].c_str(), "");
  ASSERT_STREQ(v[1].c_str(), "(123),456");
  ASSERT_STREQ(v[2].c_str(), "789,abc");
}

TEST(splitParStr, F12Point) {
  auto v6 = splitParStr(" (((1,2) , (3,4), (5,6))   ,   ((7,8) , (9,10) , (11,12))) ");
  auto v6_0 = splitParStr(v6[0]);
  auto v6_1 = splitParStr(v6[1]);

  auto v2_0_0 = splitParStr(v6_0[0]);
  auto v2_0_1 = splitParStr(v6_0[1]);
  auto v2_0_2 = splitParStr(v6_0[2]);

  auto v2_1_0 = splitParStr(v6_1[0]);
  auto v2_1_1 = splitParStr(v6_1[1]);
  auto v2_1_2 = splitParStr(v6_1[2]);

  ASSERT_STREQ(v2_0_0[0].c_str(), "1");
  ASSERT_STREQ(v2_0_0[1].c_str(), "2");
  ASSERT_STREQ(v2_0_1[0].c_str(), "3");
  ASSERT_STREQ(v2_0_1[1].c_str(), "4");
  ASSERT_STREQ(v2_0_2[0].c_str(), "5");
  ASSERT_STREQ(v2_0_2[1].c_str(), "6");
  ASSERT_STREQ(v2_1_0[0].c_str(), "7");
  ASSERT_STREQ(v2_1_0[1].c_str(), "8");
  ASSERT_STREQ(v2_1_1[0].c_str(), "9");
  ASSERT_STREQ(v2_1_1[1].c_str(), "10");
  ASSERT_STREQ(v2_1_2[0].c_str(), "11");
  ASSERT_STREQ(v2_1_2[1].c_str(), "12");
 
}

}  // namespace

int main(int argc, char **argv) {
  ::testing::InitGoogleTest(&argc, argv);
  return RUN_ALL_TESTS();
}
