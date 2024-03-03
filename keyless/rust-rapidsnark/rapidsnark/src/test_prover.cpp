#include <iostream>
#include <sstream>
#include <iomanip>
#include <string>
#include <cstdint>
#include <cstring>
#include "fr.hpp"
#include "fq.hpp"

int tests_run = 0;
int tests_failed = 0;

FrElement fr_short(int32_t val)
{
    return {val, Fr_SHORT, {0, 0, 0, 0}};
}

FrElement fr_mshort(int32_t val)
{
    return {val, Fr_SHORTMONTGOMERY, {0, 0, 0, 0}};
}

FrElement fr_long(uint64_t val0, uint64_t val1 = 0, uint64_t val2 = 0, uint64_t val3 = 0)
{
    return {0, Fr_LONG, {val0, val1, val2, val3}};
}

FrElement fr_mlong(uint64_t val0, uint64_t val1 = 0, uint64_t val2 = 0, uint64_t val3 = 0)
{
    return {0, Fr_LONGMONTGOMERY, {val0, val1, val2, val3}};
}

FqElement fq_short(int32_t val)
{
    return {val, Fq_SHORT, {0, 0, 0, 0}};
}

FqElement fq_mshort(int32_t val)
{
    return {val, Fq_SHORTMONTGOMERY, {0, 0, 0, 0}};
}

FqElement fq_long(uint64_t val0, uint64_t val1 = 0, uint64_t val2 = 0, uint64_t val3 = 0)
{
    return {0, Fq_LONG, {val0, val1, val2, val3}};
}

FqElement fq_mlong(uint64_t val0, uint64_t val1 = 0, uint64_t val2 = 0, uint64_t val3 = 0)
{
    return {0, Fq_LONGMONTGOMERY, {val0, val1, val2, val3}};
}

bool is_equal(const FrRawElement a, const FrRawElement b)
{
    return std::memcmp(a, b, sizeof(FrRawElement)) == 0;
}

bool is_equal(const PFrElement a, const PFrElement b)
{
    return std::memcmp(a, b, sizeof(FrElement)) == 0;
}

bool is_equal(const PFqElement a, const PFqElement b)
{
    return std::memcmp(a, b, sizeof(FqElement)) == 0;
}

std::string format(uint64_t val)
{
    std::ostringstream  oss;

    oss << "0x" << std::hex << std::setw(16) << std::setfill('0') << val;

    return oss.str();
}

std::string format(uint32_t val)
{
    std::ostringstream  oss;

    oss << "0x" << std::hex << std::setw(8) << std::setfill('0') << val;

    return oss.str();
}

std::string format(int32_t val)
{
    std::ostringstream  oss;

    oss << "0x" << std::hex << std::setw(8) << std::setfill('0') << val;

    return oss.str();
}

std::ostream& operator<<(std::ostream& os, const FrRawElement val)
{
    os << format(val[0]) << ","
       << format(val[1]) << ","
       << format(val[2]) << ","
       << format(val[3]);

    return os;
}

std::ostream& operator<<(std::ostream& os, const PFrElement val)
{
    os  << format(val->shortVal) << ", "
        << format(val->type)     << ", "
        << val->longVal;

    return os;
}

std::ostream& operator<<(std::ostream& os, const PFqElement val)
{
    os  << format(val->shortVal) << ", "
        << format(val->type)     << ", "
        << val->longVal;

    return os;
}

template <typename T1, typename T2, typename T3>
void compare_Result(const T1 expected, const T1 computed, const T2 A, const T3 B, int idx, std::string TestName)
{
    if (!is_equal(expected, computed))
    {
        std::cout << TestName << ":" << idx << " failed!" << std::endl;
        std::cout << "A: " << A << std::endl;
        std::cout << "B: " << B << std::endl;
        std::cout << "Expected: " << expected << std::endl;
        std::cout << "Computed: " << computed << std::endl;
        std::cout << std::endl;
        tests_failed++;
    }

    tests_run++;
}

template <typename T1, typename T2>
void compare_Result(const T1 expected, const T1 computed, const T2 A, int idx, std::string test_name)
{
    if (!is_equal(expected, computed))
    {
        std::cout << test_name << ":" << idx << " failed!" << std::endl;
        std::cout << "A: " << A << std::endl;
        std::cout << "Expected: " << expected << std::endl;
        std::cout << "Computed: " << computed << std::endl;
        std::cout << std::endl;
        tests_failed++;
    }

    tests_run++;
}

void Fr_Rw_Neg_unit_test()
{
    //Fr_Rw_Neg_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0xa1f0fac9f8000001,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    //Fr_Rw_Neg_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_Neg_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x43e1f593f0000003,0x2833e84879b97090,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_Neg_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0x43e1f593f0000003,0x2833e84879b97092,0xb85045b68181585e,0x30644e72e131a02a}; 
    //Fr_Rw_Neg_test 5:
    FrRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FrRawElement pRawResult5= {0x0,0x0,0x0,0x0};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;
    FrRawElement pRawResult5_c;

    Fr_rawNeg(pRawResult0_c, pRawA0);
    Fr_rawNeg(pRawResult1_c, pRawA1);
    Fr_rawNeg(pRawResult2_c, pRawA2);
    Fr_rawNeg(pRawResult3_c, pRawA3);
    Fr_rawNeg(pRawResult5_c, pRawA5);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_Rw_Neg_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_Rw_Neg_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_Rw_Neg_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_Rw_Neg_unit_test");
    compare_Result(pRawResult5, pRawResult5_c, pRawA5, pRawA5, 5, "Fr_Rw_Neg_unit_test");
}

void Fr_Rw_copy_unit_test()
{
    //Fr_Rw_copy_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    //Fr_Rw_copy_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x1,0x0,0x0,0x0};
    //Fr_Rw_copy_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0xfffffffffffffffe,0x0,0x0,0x0};
    //Fr_Rw_copy_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;

    Fr_rawCopy(pRawResult0_c, pRawA0);
    Fr_rawCopy(pRawResult1_c, pRawA1);
    Fr_rawCopy(pRawResult2_c, pRawA2);
    Fr_rawCopy(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_Rw_copy_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_Rw_copy_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_Rw_copy_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_Rw_copy_unit_test");
}


void Fr_Rw_add_unit_test()
{
    //Fr_rawAdd Test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FrRawElement pRawResult0= {0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba};
    //Fr_rawAdd Test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x3,0x0,0x0,0x0};
    //Fr_rawAdd Test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0xfffffffffffffffd,0x1,0x0,0x0};
    //Fr_rawAdd Test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement pRawResult3= {0xbc1e0a6c0ffffffc,0xd7cc17b786468f6d,0x47afba497e7ea7a1,0xcf9bb18d1ece5fd5};
    //Fr_rawAdd Test 6:
    FrRawElement pRawA6= {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    FrRawElement pRawB6= {0x0,0x0,0x0,0x0};
    FrRawElement pRawResult6= {0x0,0x0,0x0,0x0};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;
    FrRawElement pRawResult6_c;

    Fr_rawAdd(pRawResult0_c, pRawA0, pRawB0);
    Fr_rawAdd(pRawResult1_c, pRawA1, pRawB1);
    Fr_rawAdd(pRawResult2_c, pRawA2, pRawB2);
    Fr_rawAdd(pRawResult3_c, pRawA3, pRawB3);
    Fr_rawAdd(pRawResult6_c, pRawA6, pRawB6);


    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawB0, 0, "Fr_Rw_add_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawB1, 1, "Fr_Rw_add_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawB2, 2, "Fr_Rw_add_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawB3, 3, "Fr_Rw_add_unit_test");
    compare_Result(pRawResult6, pRawResult6_c, pRawA6, pRawB6, 6, "Fr_Rw_add_unit_test");
}

void Fr_Rw_sub_unit_test()
{
    //Fr_Rw_sub_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FrRawElement pRawResult0= {0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f};
    //Fr_Rw_sub_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_sub_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_sub_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement pRawResult3= {0x43e1f593f0000000,0x2833e84879b97090,0xb85045b68181585c,0x30644e72e131a028};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;

    Fr_rawSub(pRawResult0_c, pRawA0, pRawB0);
    Fr_rawSub(pRawResult1_c, pRawA1, pRawB1);
    Fr_rawSub(pRawResult2_c, pRawA2, pRawB2);
    Fr_rawSub(pRawResult3_c, pRawA3, pRawB3);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawB0, 0, "Fr_Rw_sub_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawB1, 1, "Fr_Rw_sub_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawB2, 2, "Fr_Rw_sub_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawB3, 3, "Fr_Rw_sub_unit_test");


}

void Fr_Rw_mul_unit_test()
{
    //Fr_Rw_mul_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FrRawElement pRawResult0= {0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d};
    //Fr_Rw_mul_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39};
    //Fr_Rw_mul_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x8663902cfae5d423,0x95d2440ac403ddd3,0x1ad411b88e349a0f,0x1ebf106109e4fa8d};
    //Fr_Rw_mul_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement pRawResult3= {0xd13604f1e300865c,0xba58b3d2a99f4ba5,0x1b4e415146d47f95,0x55c593ff9cfbf0a};
    //Fr_Rw_mul_test 4:
    FrRawElement pRawA4= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB4= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult4= {0x1d0a8ff4c8e5744c,0x6fd9959908f97ec,0xdfe72d24fcdef34e,0xd1c7f8bb929dbb};
    //Fr_Rw_mul_test 5:
    FrRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FrRawElement pRawB5= {0x2,0x0,0x0,0x0};
    FrRawElement pRawResult5= {0x0,0x0,0x0,0x0};   
    //Fr_Rw_mul_test 8:
    FrRawElement pRawA8= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB8= {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    FrRawElement pRawResult8= {0x0,0x0,0x0,0x0};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;
    FrRawElement pRawResult4_c;
    FrRawElement pRawResult5_c;
    FrRawElement pRawResult8_c;

    Fr_rawMMul(pRawResult0_c, pRawA0, pRawB0);
    Fr_rawMMul(pRawResult1_c, pRawA1, pRawB1);
    Fr_rawMMul(pRawResult2_c, pRawA2, pRawB2);
    Fr_rawMMul(pRawResult3_c, pRawA3, pRawB3);
    Fr_rawMMul(pRawResult4_c, pRawA4, pRawB4);
    Fr_rawMMul(pRawResult5_c, pRawA5, pRawB5);
    Fr_rawMMul(pRawResult8_c, pRawA8, pRawB8);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawB0, 0, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawB1, 1, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawB2, 2, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawB3, 3, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA4, pRawB4, 4, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA5, pRawB5, 5, "Fr_Rw_mul_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA8, pRawB8, 8, "Fr_Rw_mul_unit_test");


}

void Fr_Rw_Msquare_unit_test()
{
    //Fr_Rw_Msquare_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0x9907e2cb536c4654,0xd65db18eb521336a,0xe31a6546c6ec385,0x1dad258dd14a255c};
    //Fr_Rw_Msquare_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0xdc5ba0056db1194e,0x90ef5a9e111ec87,0xc8260de4aeb85d5d,0x15ebf95182c5551c};
    //Fr_Rw_Msquare_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0xa36e2021c3cb4871,0x9ccfdd64549375be,0xfabb3edd8b138d5d,0x1f90d859c5779848};
    //Fr_Rw_Msquare_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0x3ff409a0d3b30d18,0xca2027949dd16d47,0x6c8c4187ce125dad,0x3b5af5c48558e40};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;

    Fr_rawMSquare(pRawResult0_c, pRawA0);
    Fr_rawMSquare(pRawResult1_c, pRawA1);
    Fr_rawMSquare(pRawResult2_c, pRawA2);
    Fr_rawMSquare(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_Rw_Msquare_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_Rw_Msquare_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_Rw_Msquare_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_Rw_Msquare_unit_test");
}

void Fr_Rw_mul1_unit_test()
{
    //Fr_Rw_mul1_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FrRawElement pRawResult0= {0xf599ddfbad86bc06,0xec1c0a17893c85cd,0x5d482c29ab80ec64,0x4d4face96bf58f3};
    //Fr_Rw_mul1_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39};
    //Fr_Rw_mul1_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x8663902cfae5d423,0x95d2440ac403ddd3,0x1ad411b88e349a0f,0x1ebf106109e4fa8d};
    //Fr_Rw_mul1_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement pRawResult3= {0x35f905313fdf50bb,0x5bab176e33b97efa,0xafd63944c55782d,0x1402c8cfdb71d335};    
    //Fr_Rw_mul1_test 9:
    FrRawElement pRawA9= {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    FrRawElement pRawB9= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult9= {0x0,0x0,0x0,0x0};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;
    FrRawElement pRawResult9_c;

    Fr_rawMMul1(pRawResult0_c, pRawA0, pRawB0[0]);
    Fr_rawMMul1(pRawResult1_c, pRawA1, pRawB1[0]);
    Fr_rawMMul1(pRawResult2_c, pRawA2, pRawB2[0]);
    Fr_rawMMul1(pRawResult3_c, pRawA3, pRawB3[0]);
    Fr_rawMMul1(pRawResult9_c, pRawA9, pRawB9[0]);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawB0, 0, "Fr_Rw_mul1_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawB1, 1, "Fr_Rw_mul1_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawB2, 2, "Fr_Rw_mul1_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawB3, 3, "Fr_Rw_mul1_unit_test");
    compare_Result(pRawResult9, pRawResult9_c, pRawA9, pRawB9, 9, "Fr_Rw_mul1_unit_test");

}

void Fr_Rw_ToMontgomery_unit_test()
{
    //Fr_Rw_ToMontgomery_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d};
    //Fr_Rw_ToMontgomery_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0xac96341c4ffffffb,0x36fc76959f60cd29,0x666ea36f7879462e,0xe0a77c19a07df2f};
    //Fr_Rw_ToMontgomery_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x5b9a85c0dc5fb590,0x293a0258129f96b,0xd31fd70514055493,0x546132966296a07};
    //Fr_Rw_ToMontgomery_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0x8eaddd03c0bcc45a,0x1d0775cf53f57853,0xacb9a1fdb8079310,0x1b7838d45d9b3577};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;

    Fr_rawToMontgomery(pRawResult0_c, pRawA0);
    Fr_rawToMontgomery(pRawResult1_c, pRawA1);
    Fr_rawToMontgomery(pRawResult2_c, pRawA2);
    Fr_rawToMontgomery(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_Rw_ToMontgomery_unit_test");
}

void Fr_Rw_IsEq_unit_test()
{
    //Fr_rawIsEq 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FrRawElement pRawResult0= {0x0};
    //Fr_rawIsEq 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawB1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x1};
    //Fr_rawIsEq 2:
    FrRawElement pRawA2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x1};
    //Fr_rawIsEq 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement pRawResult3= {0x0};

    //Fr_rawIsEq 7:
    FrRawElement pRawA7= {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    FrRawElement pRawB7= {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};
    FrRawElement pRawResult7= {0x1};

    FrRawElement pRawResult0_c = {0};
    FrRawElement pRawResult1_c = {0};
    FrRawElement pRawResult2_c = {0};
    FrRawElement pRawResult3_c = {0};
    FrRawElement pRawResult7_c = {0};

    pRawResult0_c[0] = Fr_rawIsEq(pRawA0, pRawB0);
    pRawResult1_c[0] = Fr_rawIsEq(pRawA1, pRawB1);
    pRawResult2_c[0] = Fr_rawIsEq(pRawA2, pRawB2);
    pRawResult3_c[0] = Fr_rawIsEq(pRawA3, pRawB3);
    pRawResult7_c[0] = Fr_rawIsEq(pRawA7, pRawB7);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawB0, 0, "Fr_Rw_IsEq_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawB1, 1, "Fr_Rw_IsEq_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawB2, 2, "Fr_Rw_IsEq_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawB3, 3, "Fr_Rw_IsEq_unit_test");
    compare_Result(pRawResult7, pRawResult7_c, pRawA7, pRawB7, 7, "Fr_Rw_IsEq_unit_test");
}

void Fr_rawIsZero_unit_test()
{
    //Fr_rawIsZero_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0x0};
    //Fr_rawIsZero_test 1:
    FrRawElement pRawA1= {0x0,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0x1};
    //Fr_rawIsZero_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x0};
    //Fr_rawIsZero_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0x0};

    //Fr_rawIsZero_test 5:
    FrRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FrRawElement pRawResult5= {0x1};

    FrRawElement pRawResult0_c = {0};
    FrRawElement pRawResult1_c = {0};
    FrRawElement pRawResult2_c = {0};
    FrRawElement pRawResult3_c = {0};
    FrRawElement pRawResult5_c = {0};

    pRawResult0_c[0] = Fr_rawIsZero(pRawA0);
    pRawResult1_c[0] = Fr_rawIsZero(pRawA1);
    pRawResult2_c[0] = Fr_rawIsZero(pRawA2);
    pRawResult3_c[0] = Fr_rawIsZero(pRawA3);
    pRawResult5_c[0] = Fr_rawIsZero(pRawA5);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_rawIsZero_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_rawIsZero_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_rawIsZero_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_rawIsZero_unit_test");
    compare_Result(pRawResult5, pRawResult5_c, pRawA5, pRawA5, 5, "Fr_rawIsZero_unit_test");
}

void Fr_Rw_FromMontgomery_unit_test()
{
    //Fr_Rw_FromMontgomery_test 0:
    FrRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FrRawElement pRawResult0= {0x55b425913927735a,0xa3ac6d7389307a4d,0x543d3ec42a2529ae,0x256e51ca1fcef59b};
    //Fr_Rw_FromMontgomery_test 1:
    FrRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FrRawElement pRawResult1= {0xdc5ba0056db1194e,0x90ef5a9e111ec87,0xc8260de4aeb85d5d,0x15ebf95182c5551c};
    //Fr_Rw_FromMontgomery_test 2:
    FrRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FrRawElement pRawResult2= {0x26d7659f271a8bb3,0x21364eeee929d8a6,0xd869189184a2650f,0x2f92867a259f026d};
    //Fr_Rw_FromMontgomery_test 3:
    FrRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FrRawElement pRawResult3= {0x3114fb0a8790445e,0x3c686fb82b0dbda3,0xa509fd6ff15d77e,0x247132c3c886548};

    FrRawElement pRawResult0_c;
    FrRawElement pRawResult1_c;
    FrRawElement pRawResult2_c;
    FrRawElement pRawResult3_c;

    Fr_rawFromMontgomery(pRawResult0_c, pRawA0);
    Fr_rawFromMontgomery(pRawResult1_c, pRawA1);
    Fr_rawFromMontgomery(pRawResult2_c, pRawA2);
    Fr_rawFromMontgomery(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c, pRawA0, pRawA0, 0, "Fr_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult1, pRawResult1_c, pRawA1, pRawA1, 1, "Fr_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult2, pRawResult2_c, pRawA2, pRawA2, 2, "Fr_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult3, pRawResult3_c, pRawA3, pRawA3, 3, "Fr_Rw_FromMontgomery_unit_test");
}

void Fr_copy_unit_test()
{
    //Fr_copy_test 0:
    FrElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_copy_test 1:
    FrElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_copy_test 2:
    FrElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_copy_test 3:
    FrElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_copy(&Result0_c, &pA0);
    Fr_copy(&Result1_c, &pA1);
    Fr_copy(&Result2_c, &pA2);
    Fr_copy(&Result3_c, &pA3);

    compare_Result(&pResult0, &Result0_c, &pA0, &pA0, 0, "Fr_copy_unit_test");
    compare_Result(&pResult1, &Result1_c, &pA1, &pA1, 1, "Fr_copy_unit_test");
    compare_Result(&pResult2, &Result2_c, &pA2, &pA2, 2, "Fr_copy_unit_test");
    compare_Result(&pResult3, &Result3_c, &pA3, &pA3, 3, "Fr_copy_unit_test");
}

void Fr_copyn_unit_test()
{
    //Fr_copy_test 0:
    FrElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_copy_test 1:
    FrElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_copy_test 2:
    FrElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_copy_test 3:
    FrElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult3= {0x0,0x0,{0x0,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_copyn(&Result0_c, &pA0,0);
    Fr_copyn(&Result1_c, &pA1,1);
    Fr_copyn(&Result2_c, &pA2,1);
    Fr_copyn(&Result3_c, &pA3,0);

    compare_Result(&pResult0, &Result0_c, &pA0, &pA0, 0, "Fr_copyn_unit_test");
    compare_Result(&pResult1, &Result1_c, &pA1, &pA1, 1, "Fr_copyn_unit_test");
    compare_Result(&pResult2, &Result2_c, &pA2, &pA2, 2, "Fr_copyn_unit_test");
    compare_Result(&pResult3, &Result3_c, &pA3, &pA3, 3, "Fr_copyn_unit_test");
}

void Fq_copy_unit_test()
{
    //Fq_copy_test 0:
    FqElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_copy_test 1:
    FqElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_copy_test 2:
    FqElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_copy_test 3:
    FqElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_copy(&Result0_c, &pA0);
    Fq_copy(&Result1_c, &pA1);
    Fq_copy(&Result2_c, &pA2);
    Fq_copy(&Result3_c, &pA3);

    compare_Result(&pResult0, &Result0_c, &pA0, &pA0, 0, "Fq_copy_unit_test");
    compare_Result(&pResult1, &Result1_c, &pA1, &pA1, 1, "Fq_copy_unit_test");
    compare_Result(&pResult2, &Result2_c, &pA2, &pA2, 2, "Fq_copy_unit_test");
    compare_Result(&pResult3, &Result3_c, &pA3, &pA3, 3, "Fq_copy_unit_test");
}

void Fq_copyn_unit_test()
{
    //Fq_copy_test 0:
    FqElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_copy_test 1:
    FqElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_copy_test 2:
    FqElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_copy_test 3:
    FqElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult3= {0x0,0x0,{0x0,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_copyn(&Result0_c, &pA0,0);
    Fq_copyn(&Result1_c, &pA1,1);
    Fq_copyn(&Result2_c, &pA2,1);
    Fq_copyn(&Result3_c, &pA3,0);

    compare_Result(&pResult0, &Result0_c, &pA0, &pA0, 0, "Fq_copyn_unit_test");
    compare_Result(&pResult1, &Result1_c, &pA1, &pA1, 1, "Fq_copyn_unit_test");
    compare_Result(&pResult2, &Result2_c, &pA2, &pA2, 2, "Fq_copyn_unit_test");
    compare_Result(&pResult3, &Result3_c, &pA3, &pA3, 3, "Fq_copyn_unit_test");
}

void Fr_toNormal_unit_test()
{
    //Fr_toNormal_test 0:
    FrElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_toNormal_test 1:
    FrElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_toNormal_test 2:
    FrElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_toNormal_test 3:
    FrElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult3= {0x0,0x80000000,{0x55b425913927735a,0xa3ac6d7389307a4d,0x543d3ec42a2529ae,0x256e51ca1fcef59b}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_toNormal(&Result0_c, &pA0);
    Fr_toNormal(&Result1_c, &pA1);
    Fr_toNormal(&Result2_c, &pA2);
    Fr_toNormal(&Result3_c, &pA3);

    compare_Result(&pResult0, &Result0_c, &pA0, &pA0, 0, "Fr_toNormal_unit_test");
    compare_Result(&pResult1, &Result1_c, &pA1, &pA1, 1, "Fr_toNormal_unit_test");
    compare_Result(&pResult2, &Result2_c, &pA2, &pA2, 2, "Fr_toNormal_unit_test");
    compare_Result(&pResult3, &Result3_c, &pA3, &pA3, 3, "Fr_toNormal_unit_test");
}

void Fr_mul_s1s2_unit_test()
{
    //Fr_mul_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_mul_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x0,0x80000000,{0x1188b480,0x0,0x0,0x0}};
    //Fr_mul_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x80000000,{0x3fffffff00000001,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_mul(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_mul(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_mul(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c, &pA_s1s20, &pB_s1s20, 0, "Fr_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c, &pA_s1s21, &pB_s1s21, 1, "Fr_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c, &pA_s1s22, &pB_s1s22, 2, "Fr_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c, &pA_s1s23, &pB_s1s23, 3, "Fr_mul_s1s2_unit_test");
}

void Fr_mul_l1nl2n_unit_test()
{
    //Fr_mul_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0xc0000000,{0x592c68389ffffff6,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_mul_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0xc0000000,{0x1497892315a07fe1,0x930f99e96b3b9535,0x73b1e28430b17066,0x29e821cd214b6d6b}};
    //Fr_mul_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0xc0000000,{0x19094ca438fc19d0,0x4f1502bc99846068,0x5cc3236f2303a977,0x17808a731cd75a48}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_mul(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_mul(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_mul(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c, &pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c, &pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c, &pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c, &pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_mul_l1nl2n_unit_test");
}

void Fr_mul_l1ml2n_unit_test()
{
    //Fr_mul_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0x80000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0x80000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_mul(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_mul(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_mul(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_mul_l1ml2n_unit_test");
}

void Fr_mul_l1ml2m_unit_test()
{
    //Fr_mul_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0xc0000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0xc0000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0xc0000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_mul(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_mul(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_mul(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_mul_l1ml2m_unit_test");
}

void Fr_mul_l1nl2m_unit_test()
{
    //Fr_mul_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0x80000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0x80000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_mul(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_mul(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_mul(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_mul_l1nl2m_unit_test");
}

void Fr_mul_l1ns2n_unit_test()
{
    //Fr_mul_l1ns2n_test 0:
    FrElement pA_l1ns2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns2n0= {0x0,0xc0000000,{0x592c68389ffffff6,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_mul_l1ns2n_test 1:
    FrElement pA_l1ns2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ns2n_test 2:
    FrElement pA_l1ns2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns2n2= {0x0,0xc0000000,{0x2d67d8d2e0004952,0xaddd11ecde7f7ae3,0xed975f635da0de4d,0x1a7fe303489132eb}};
    //Fr_mul_l1ns2n_test 3:
    FrElement pA_l1ns2n3= {0x7fffffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns2n3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns2n3= {0x0,0xc0000000,{0x90dd4dd6a1de9254,0xe2fe3be3bc047346,0xda25203224bdc5a8,0xbf3a7101ab99a89}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ns2n0, &pB_l1ns2n0);
    Fr_mul(&Result1_c, &pA_l1ns2n1, &pB_l1ns2n1);
    Fr_mul(&Result2_c, &pA_l1ns2n2, &pB_l1ns2n2);
    Fr_mul(&Result3_c, &pA_l1ns2n3, &pB_l1ns2n3);

    compare_Result(&pResult_l1ns2n0, &Result0_c,&pA_l1ns2n0, &pB_l1ns2n0, 0, "Fr_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n1, &Result1_c,&pA_l1ns2n1, &pB_l1ns2n1, 1, "Fr_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n2, &Result2_c,&pA_l1ns2n2, &pB_l1ns2n2, 2, "Fr_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n3, &Result3_c,&pA_l1ns2n3, &pB_l1ns2n3, 3, "Fr_mul_l1ns2n_unit_test");
}

void Fr_mul_s1nl2n_unit_test()
{
    //Fr_mul_s1nl2n_test 0:
    FrElement pA_s1nl2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2n0= {0x0,0xc0000000,{0x592c68389ffffff6,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_mul_s1nl2n_test 1:
    FrElement pA_s1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_s1nl2n_test 2:
    FrElement pA_s1nl2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1nl2n2= {0x0,0xc0000000,{0x3c79e7002385099,0x69bfe0da5a608f7b,0x3dbd93ce32b4e2b3,0x773561b6a940451}};
    //Fr_mul_s1nl2n_test 3:
    FrElement pA_s1nl2n3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1nl2n3= {0x7fffffff,0x80000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_s1nl2n3= {0x0,0xc0000000,{0x7c8b07120fa19dd4,0x19b02d60cfbeb467,0xe1f374b7a57d8ed5,0x22a83208b264056d}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_s1nl2n0, &pB_s1nl2n0);
    Fr_mul(&Result1_c, &pA_s1nl2n1, &pB_s1nl2n1);
    Fr_mul(&Result2_c, &pA_s1nl2n2, &pB_s1nl2n2);
    Fr_mul(&Result3_c, &pA_s1nl2n3, &pB_s1nl2n3);

    compare_Result(&pResult_s1nl2n0, &Result0_c,&pA_s1nl2n0, &pB_s1nl2n0, 0, "Fr_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n1, &Result1_c,&pA_s1nl2n1, &pB_s1nl2n1, 1, "Fr_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n2, &Result2_c,&pA_s1nl2n2, &pB_s1nl2n2, 2, "Fr_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n3, &Result3_c,&pA_s1nl2n3, &pB_s1nl2n3, 3, "Fr_mul_s1nl2n_unit_test");
}

void Fr_mul_s1nl2m_unit_test()
{
    //Fr_mul_s1nl2m_test 0:
    FrElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_s1nl2m_test 1:
    FrElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_s1nl2m_test 2:
    FrElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1nl2m2= {0x0,0x80000000,{0xd708561abffca754,0x6c6d984a2702a36a,0xc0f6e8587da122fb,0x164b29d2c31ce3ab}};
    //Fr_mul_s1nl2m_test 3:
    FrElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_s1nl2m3= {0x0,0x80000000,{0xab57780eac37ddd2,0x9ffb06c643291bf,0xb327f5cb01f66c9e,0x2f40c4dcc2ed6d85}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fr_mul(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fr_mul(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fr_mul(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fr_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fr_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fr_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fr_mul_s1nl2m_unit_test");
}

void Fr_mul_l1ms2n_unit_test()
{
    //Fr_mul_l1ms2n_test 0:
    FrElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1ms2n_test 1:
    FrElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ms2n_test 2:
    FrElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2n2= {0x0,0x80000000,{0x5d70bdff3d855140,0xfab648d14060e580,0xc8cd54f7f14513ba,0x23995be618ce6b27}};
    //Fr_mul_l1ms2n_test 3:
    FrElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_l1ms2n3= {0x0,0x80000000,{0xab57780eac37ddd2,0x9ffb06c643291bf,0xb327f5cb01f66c9e,0x2f40c4dcc2ed6d85}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fr_mul(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fr_mul(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fr_mul(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fr_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fr_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fr_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fr_mul_l1ms2n_unit_test");
}

void Fr_mul_l1ns2m_unit_test()
{
    //Fr_mul_l1ns2m_test 0:
    FrElement pA_l1ns2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns2m0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1ns2m_test 1:
    FrElement pA_l1ns2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ns2m_test 2:
    FrElement pA_l1ns2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns2m2= {0x0,0x80000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_l1ns2m_test 3:
    FrElement pA_l1ns2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns2m3= {0x0,0x80000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ns2m0, &pB_l1ns2m0);
    Fr_mul(&Result1_c, &pA_l1ns2m1, &pB_l1ns2m1);
    Fr_mul(&Result2_c, &pA_l1ns2m2, &pB_l1ns2m2);
    Fr_mul(&Result3_c, &pA_l1ns2m3, &pB_l1ns2m3);

    compare_Result(&pResult_l1ns2m0, &Result0_c,&pA_l1ns2m0, &pB_l1ns2m0, 0, "Fr_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m1, &Result1_c,&pA_l1ns2m1, &pB_l1ns2m1, 1, "Fr_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m2, &Result2_c,&pA_l1ns2m2, &pB_l1ns2m2, 2, "Fr_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m3, &Result3_c,&pA_l1ns2m3, &pB_l1ns2m3, 3, "Fr_mul_l1ns2m_unit_test");
}

void Fr_mul_l1ms2m_unit_test()
{
    //Fr_mul_l1ms2m_test 0:
    FrElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m0= {0x0,0xc0000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_l1ms2m_test 1:
    FrElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_l1ms2m_test 2:
    FrElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2m2= {0x0,0xc0000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_l1ms2m_test 3:
    FrElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms2m3= {0x0,0xc0000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fr_mul(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fr_mul(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fr_mul(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fr_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fr_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fr_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fr_mul_l1ms2m_unit_test");
}

void Fr_mul_s1ml2m_unit_test()
{
    //Fr_mul_s1ml2m_test 0:
    FrElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m0= {0x0,0xc0000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_s1ml2m_test 1:
    FrElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_s1ml2m_test 2:
    FrElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1ml2m2= {0x0,0xc0000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_s1ml2m_test 3:
    FrElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1ml2m3= {0x0,0xc0000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fr_mul(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fr_mul(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fr_mul(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fr_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fr_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fr_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fr_mul_s1ml2m_unit_test");
}

void Fr_mul_s1ml2n_unit_test()
{
    //Fr_mul_s1ml2n_test 0:
    FrElement pA_s1ml2n0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2n0= {0x0,0x80000000,{0xb8b7400adb62329c,0x121deb53c223d90f,0x904c1bc95d70baba,0x2bd7f2a3058aaa39}};
    //Fr_mul_s1ml2n_test 1:
    FrElement pA_s1ml2n1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_mul_s1ml2n_test 2:
    FrElement pA_s1ml2n2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1ml2n2= {0x0,0x80000000,{0xcba5e0bbd0000003,0x789bb8d96d2c51b3,0x28f0d12384840917,0x112ceb58a394e07d}};
    //Fr_mul_s1ml2n_test 3:
    FrElement pA_s1ml2n3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1ml2n3= {0x0,0x80000000,{0xdea6a001d841e408,0xffd01934b5bef5d1,0xedc4ef0cf4a2b471,0x1d8f65bdb91d796f}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_mul(&Result0_c, &pA_s1ml2n0, &pB_s1ml2n0);
    Fr_mul(&Result1_c, &pA_s1ml2n1, &pB_s1ml2n1);
    Fr_mul(&Result2_c, &pA_s1ml2n2, &pB_s1ml2n2);
    Fr_mul(&Result3_c, &pA_s1ml2n3, &pB_s1ml2n3);

    compare_Result(&pResult_s1ml2n0, &Result0_c,&pA_s1ml2n0, &pB_s1ml2n0, 0, "Fr_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n1, &Result1_c,&pA_s1ml2n1, &pB_s1ml2n1, 1, "Fr_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n2, &Result2_c,&pA_s1ml2n2, &pB_s1ml2n2, 2, "Fr_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n3, &Result3_c,&pA_s1ml2n3, &pB_s1ml2n3, 3, "Fr_mul_s1ml2n_unit_test");
}

void Fr_sub_s1s2_unit_test()
{
    //Fr_sub_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {-1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_sub_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {-2,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_sub_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x8638,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_sub_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_sub(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_sub(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_sub(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_sub_s1s2_unit_test");
}

void Fr_sub_l1nl2n_unit_test()
{
    //Fr_sub_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x80000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x80000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0x80000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fr_sub_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_sub(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_sub(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_sub(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_sub_l1nl2n_unit_test");
}

void Fr_sub_l1ml2n_unit_test()
{
    //Fr_sub_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0xc0000000,{0xeab58d5b5000000c,0xba3afb1d3af7d63d,0xeb72fed7908ecc00,0x144f5eefad21e1ca}};
    //Fr_sub_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0xc0000000,{0xeab58d5b5000000b,0xba3afb1d3af7d63d,0xeb72fed7908ecc00,0x144f5eefad21e1ca}};
    //Fr_sub_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0xc0000000,{0x435c21e84340ffc0,0x69d157661fe10190,0x52eb5c769f20dc41,0xb39cdedf0cc6a98}};
    //Fr_sub_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0xc0000000,{0x4cfb5842b1de9252,0xbaca539b424b02b5,0x21d4da7ba33c6d4b,0xdb8f589d3987fa60}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_sub(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_sub(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_sub(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_sub_l1ml2n_unit_test");
}

void Fr_sub_l1ml2m_unit_test()
{
    //Fr_sub_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0xc0000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0xc0000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fr_sub_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_sub(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_sub(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_sub(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_sub_l1ml2m_unit_test");
}

void Fr_sub_l1nl2m_unit_test()
{
    //Fr_sub_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0xc0000000,{0xac96341c4ffffff9,0x36fc76959f60cd29,0x666ea36f7879462e,0xe0a77c19a07df2f}};
    //Fr_sub_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0xc0000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0xc0000000,{0xafecfa7621de925c,0x249d7e2789cff7d0,0x9ca74de630c88892,0xf161aa724469bd7}};
    //Fr_sub_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0xc0000000,{0xf6e69d513e216daf,0x6d6994ad376e6ddb,0x967b6b3ade44eb11,0x54d4f5d5a7a9a5c9}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_sub(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_sub(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_sub(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_sub_l1nl2m_unit_test");
}

void Fr_sub_s1nl2m_unit_test()
{
    //Fr_sub_s1nl2m_test 0:
    FrElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m0= {0x0,0xc0000000,{0xac96341c4ffffff9,0x36fc76959f60cd29,0x666ea36f7879462e,0xe0a77c19a07df2f}};
    //Fr_sub_s1nl2m_test 1:
    FrElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m1= {0x0,0xc0000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1nl2m_test 2:
    FrElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1nl2m2= {0x0,0xc0000000,{0xbb4f6fd511db39ad,0x186f5d9843a64987,0x34ad651b29e5a276,0x1434592143ce9f06}};
    //Fr_sub_s1nl2m_test 3:
    FrElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_s1nl2m3= {0x0,0xc0000000,{0x5b2db70b90000008,0x996b59fb541213f9,0x8a31e7fd8a896a8c,0xd2be2524285b6124}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fr_sub(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fr_sub(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fr_sub(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fr_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fr_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fr_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fr_sub_s1nl2m_unit_test");
}

void Fr_sub_l1ms2n_unit_test()
{
    //Fr_sub_l1ms2n_test 0:
    FrElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n0= {0x0,0xc0000000,{0xeab58d5b5000000c,0xba3afb1d3af7d63d,0xeb72fed7908ecc00,0x144f5eefad21e1ca}};
    //Fr_sub_l1ms2n_test 1:
    FrElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n1= {0x0,0xc0000000,{0xeab58d5b5000000b,0xba3afb1d3af7d63d,0xeb72fed7908ecc00,0x144f5eefad21e1ca}};
    //Fr_sub_l1ms2n_test 2:
    FrElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2n2= {0x0,0xc0000000,{0xb8deb6dbc80092a3,0xc7a02fb580223d7d,0xff069beb7a81106c,0x1ccd9ecd208995c2}};
    //Fr_sub_l1ms2n_test 3:
    FrElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_l1ms2n3= {0x0,0xc0000000,{0xe8b43e885ffffff9,0x8ec88e4d25a75c97,0x2e1e5db8f6f7edd0,0x5da6294eb8d63f05}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fr_sub(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fr_sub(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fr_sub(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fr_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fr_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fr_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fr_sub_l1ms2n_unit_test");
}

void Fr_sub_l1ms2m_unit_test()
{
    //Fr_sub_l1ms2m_test 0:
    FrElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m0= {0x0,0xc0000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ms2m_test 1:
    FrElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m1= {0x0,0xc0000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ms2m_test 2:
    FrElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fr_sub_l1ms2m_test 3:
    FrElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fr_sub(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fr_sub(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fr_sub(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fr_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fr_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fr_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fr_sub_l1ms2m_unit_test");
}

void Fr_sub_s1ml2m_unit_test()
{
    //Fr_sub_s1ml2m_test 0:
    FrElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m0= {0x0,0xc0000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1ml2m_test 1:
    FrElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m1= {0x0,0xc0000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1ml2m_test 2:
    FrElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1ml2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fr_sub_s1ml2m_test 3:
    FrElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1ml2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fr_sub(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fr_sub(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fr_sub(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fr_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fr_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fr_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fr_sub_s1ml2m_unit_test");
}

void Fr_sub_l1ns2_unit_test()
{
    //Fr_sub_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x80000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x80000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x0,0x80000000,{0xa1f0fac9f7ffe448,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_sub_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x80000000,{0xffffffffffff0000,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_sub(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_sub(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_sub(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c, &pA_l1ns21, &pB_l1ns21, 1, "Fr_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c, &pA_l1ns22, &pB_l1ns22, 2, "Fr_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_sub_l1ns2_unit_test");
}

void Fr_sub_s1l2n_unit_test()
{
    //Fr_sub_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x80000000,{0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x80000000,{0x43e1f593efffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x80000000,{0x28290f4e41df344a,0xd435ad96965d16ae,0x2c06c2792dc5d7d7,0x2e4d7dc161e35b84}};
    //Fr_sub_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x80000000,{0x43e1f593f0010001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
    //Fr_sub_s1l2n_test 4:
    FrElement pA_s1l2n4= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n4= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n4= {0x0,0x80000000,{0x87c3eb27e0000002,0x5067d090f372e122,0x70a08b6d0302b0ba,0x60c89ce5c2634053}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};
    FrElement Result4_c= {0,0,{0,0,0,0}};

    Fr_sub(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_sub(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_sub(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_sub(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);
    Fr_sub(&Result4_c, &pA_s1l2n4, &pB_s1l2n4);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n4, &Result4_c,&pA_s1l2n4, &pB_s1l2n4, 4, "Fr_sub_s1l2n_unit_test");
}


void Fq_sub_s1s2_unit_test()
{
    //Fq_sub_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {-1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_sub_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {-2,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_sub_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x8638,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_sub_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_sub(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_sub(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_sub(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_sub_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_sub_s1s2_unit_test");
}

void Fq_sub_l1nl2n_unit_test()
{
    //Fq_sub_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x80000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x80000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0x80000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fq_sub_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_sub(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_sub(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_sub(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_sub_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_sub_l1nl2n_unit_test");
}

void Fq_sub_l1ml2n_unit_test()
{
    //Fq_sub_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0xc0000000,{0x956604fb4d5ee20e,0x828f943f7ce3b411,0xeb72fed7908ecc05,0x144f5eefad21e1ca}};
    //Fq_sub_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0xc0000000,{0x956604fb4d5ee20d,0x828f943f7ce3b411,0xeb72fed7908ecc05,0x144f5eefad21e1ca}};
    //Fq_sub_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0xc0000000,{0xbc5da4512aea30e2,0x7e8e848102891238,0xb557a3d6f0ff1715,0x0f7a12ca382aae56}};
    //Fq_sub_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0xc0000000,{0xa40fbb1b998715cc,0xbd106785b9103eb4,0x66733eb9ecb66dd7,0xd6cd89dcee1e09e6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_sub(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_sub(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_sub(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_sub_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_sub_l1ml2n_unit_test");
}

void Fq_sub_l1ml2m_unit_test()
{
    //Fq_sub_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0xc0000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0xc0000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fq_sub_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_sub(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_sub(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_sub(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_sub_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_sub_l1ml2m_unit_test");
}

void Fq_sub_l1nl2m_unit_test()
{
    //Fq_sub_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0xc0000000,{0xd35d438dc58f0d9b,0x0a78eb28f5c70b3d,0x666ea36f7879462c,0x0e0a77c19a07df2f}};
    //Fq_sub_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0xc0000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0xc0000000,{0x4ecd11436b60a7eb,0xf5e9d1d6e9cb832d,0xac265d0c7f255fb0,0x09df617d19c47ce1}};
    //Fq_sub_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0xc0000000,{0x9810d0fb3ef5e77b,0xda71030baf618bd8,0x51dd06fc94caea85,0x5996c495f3139643}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_sub(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_sub(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_sub(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_sub_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_sub_l1nl2m_unit_test");
}

void Fq_sub_s1nl2m_unit_test()
{
    //Fq_sub_s1nl2m_test 0:
    FqElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m0= {0x0,0xc0000000,{0xd35d438dc58f0d9b,0x0a78eb28f5c70b3d,0x666ea36f7879462c,0x0e0a77c19a07df2f}};
    //Fq_sub_s1nl2m_test 1:
    FqElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m1= {0x0,0xc0000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1nl2m_test 2:
    FqElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1nl2m2= {0x0,0xc0000000,{0xa9fc967eeefefea5,0x24b7f65f72e74e2b,0x34ad651b29e42e00,0x1434592143ce9f06}};
    //Fq_sub_s1nl2m_test 3:
    FqElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_s1nl2m3= {0x0,0xc0000000,{0x24e3d49feb6aecf2,0xa489e9f9db1c89dd,0x8a31e7fd8a896a8f,0xd2be2524285b6124}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fq_sub(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fq_sub(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fq_sub(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fq_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fq_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fq_sub_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fq_sub_s1nl2m_unit_test");
}

void Fq_sub_l1ms2n_unit_test()
{
    //Fq_sub_l1ms2n_test 0:
    FqElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n0= {0x0,0xc0000000,{0x956604fb4d5ee20e,0x828f943f7ce3b411,0xeb72fed7908ecc05,0x144f5eefad21e1ca}};
    //Fq_sub_l1ms2n_test 1:
    FqElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n1= {0x0,0xc0000000,{0x956604fb4d5ee20d,0x828f943f7ce3b411,0xeb72fed7908ecc05,0x144f5eefad21e1ca}};
    //Fq_sub_l1ms2n_test 2:
    FqElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2n2= {0x0,0xc0000000,{0x7a8f34cd1807c235,0xda47bc4aa2d53c80,0xff069beb7a81502d,0x1ccd9ecd208995c2}};
    //Fq_sub_l1ms2n_test 3:
    FqElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_l1ms2n3= {0x0,0xc0000000,{0x173cb776ed121055,0xf2f780978d5540b0,0x2e1e5db8f6f7edcd,0x5da6294eb8d63f05}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fq_sub(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fq_sub(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fq_sub(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fq_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fq_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fq_sub_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fq_sub_l1ms2n_unit_test");
}

void Fq_sub_l1ms2m_unit_test()
{
    //Fq_sub_l1ms2m_test 0:
    FqElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m0= {0x0,0xc0000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ms2m_test 1:
    FqElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m1= {0x0,0xc0000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ms2m_test 2:
    FqElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fq_sub_l1ms2m_test 3:
    FqElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fq_sub(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fq_sub(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fq_sub(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fq_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fq_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fq_sub_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fq_sub_l1ms2m_unit_test");
}

void Fq_sub_s1ml2m_unit_test()
{
    //Fq_sub_s1ml2m_test 0:
    FqElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m0= {0x0,0xc0000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1ml2m_test 1:
    FqElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m1= {0x0,0xc0000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1ml2m_test 2:
    FqElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1ml2m2= {0x0,0xc0000000,{0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f}};
    //Fq_sub_s1ml2m_test 3:
    FqElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1ml2m3= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fq_sub(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fq_sub(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fq_sub(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fq_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fq_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fq_sub_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fq_sub_s1ml2m_unit_test");
}

void Fq_sub_l1ns2_unit_test()
{
    //Fq_sub_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x80000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x80000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x0,0x80000000,{0xa1f0fac9f7ffe448,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_sub_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x80000000,{0xffffffffffff0000,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_sub(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_sub(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_sub(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c, &pA_l1ns21, &pB_l1ns21, 1, "Fq_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c, &pA_l1ns22, &pB_l1ns22, 2, "Fq_sub_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_sub_l1ns2_unit_test");
}

void Fq_sub_s1l2n_unit_test()
{
    //Fq_sub_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x80000000,{0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x80000000,{0x3c208c16d87cfd45,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x80000000,{0x2067a5d12a5c3190,0x43832fdf851570aa,0x2c06c2792dc5d7d8,0x2e4d7dc161e35b84}};
    //Fq_sub_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x80000000,{0x3c208c16d87dfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
    //Fq_sub_s1l2n_test 4:
    FqElement pA_s1l2n4= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n4= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n4= {0x0,0x80000000,{0x7841182db0f9fa8e,0x2f02d522d0e3951a,0x70a08b6d0302b0bb,0x60c89ce5c2634053}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};
    FqElement Result4_c= {0,0,{0,0,0,0}};

    Fq_sub(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_sub(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_sub(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_sub(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);
    Fq_sub(&Result4_c, &pA_s1l2n4, &pB_s1l2n4);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_sub_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n4, &Result4_c,&pA_s1l2n4, &pB_s1l2n4, 4, "Fq_sub_s1l2n_unit_test");
}

void Fr_add_s1s2_unit_test()
{
    //Fr_add_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x3,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_add_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x2,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_add_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0xbda8,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_add_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x80000000,{0xfffffffe,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_add(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_add(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_add(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_add_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_add_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_add_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_add_s1s2_unit_test");
}


void Fq_add_s1s2_unit_test()
{
    //Fq_add_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x3,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_add_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x2,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_add_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0xbda8,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_add_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x80000000,{0xfffffffe,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_add(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_add(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_add(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_add_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_add_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_add_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_add_s1s2_unit_test");
}

void Fq_add_l1nl2n_unit_test()
{
    //Fq_add_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0x80000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fq_add_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x80000000,{0xc3df73e9278302b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_add(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_add(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_add(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_add_l1nl2n_unit_test");
}

void Fq_add_l1ml2n_unit_test()
{
    //Fq_add_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0xc0000000,{0xa6ba871b8b1e1b3b,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_add_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0xc0000000,{0xa6ba871b8b1e1b3a,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_add_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0xc0000000,{0x87845142c515cf1e,0xa9a563c777305e58,0x02f8a1df90824147,0x20ea3ba8a906f1d3}};
    //Fq_add_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0xc0000000,{0x1fcfb8cd8dfbeceb,0xab6e2de8de7df6be,0xe13c7b8f91c839ca,0xf8ce27b030b055ef}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_add(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_add(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_add(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_add_l1ml2n_unit_test");
}

void Fq_add_l1ml2m_unit_test()
{
    //Fq_add_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fq_add_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0xc0000000,{0xc3df73e9278302b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_add(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_add(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_add(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_add_l1ml2m_unit_test");
}

void Fq_add_l1nl2m_unit_test()
{
    //Fq_add_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0xc0000000,{0xd35d438dc58f0d9f,0x0a78eb28f5c70b3d,0x666ea36f7879462c,0x0e0a77c19a07df2f}};
    //Fq_add_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0xc0000000,{0x863eddcec7a38339,0x9de6473ab08436f3,0xc4b96387269c60bb,0x0e0d02e01861062c}};
    //Fq_add_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0xc0000000,{0x1fcfb8cd8dfbeceb,0xab6e2de8de7df6be,0xe13c7b8f91c839ca,0xf8ce27b030b055ef}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_add(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_add(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_add(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_add_l1nl2m_unit_test");
}

void Fq_add_s1nl2m_unit_test()
{
    //Fq_add_s1nl2m_test 0:
    FqElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m0= {0x0,0xc0000000,{0xd35d438dc58f0d9f,0x0a78eb28f5c70b3d,0x666ea36f7879462c,0x0e0a77c19a07df2f}};
    //Fq_add_s1nl2m_test 1:
    FqElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_s1nl2m_test 2:
    FqElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1nl2m2= {0x0,0xc0000000,{0xe16e630a4b41d9f3,0xccb46bc339a001f1,0x4d406b95d15b2f0a,0x1861fa84426b2851}};
    //Fq_add_s1nl2m_test 3:
    FqElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_s1nl2m3= {0x0,0xc0000000,{0xaca2bc723a70f262,0x758714d70a38f4c1,0x19915c908786b9d3,0x71f5883e65f820d0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fq_add(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fq_add(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fq_add(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fq_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fq_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fq_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fq_add_s1nl2m_unit_test");
}

void Fq_add_l1ms2n_unit_test()
{
    //Fq_add_l1ms2n_test 0:
    FqElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n0= {0x0,0xc0000000,{0xa6ba871b8b1e1b3b,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_add_l1ms2n_test 1:
    FqElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n1= {0x0,0xc0000000,{0xa6ba871b8b1e1b3a,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_add_l1ms2n_test 2:
    FqElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2n2= {0x0,0xc0000000,{0xc952c0c6d7f83dcb,0x4dec2bfdd6e43410,0xb949a9cb0700082f,0x1396afa5c0a80a66}};
    //Fq_add_l1ms2n_test 3:
    FqElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_l1ms2n3= {0x0,0xc0000000,{0xaca2bc723a70f262,0x758714d70a38f4c1,0x19915c908786b9d3,0x71f5883e65f820d0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fq_add(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fq_add(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fq_add(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fq_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fq_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fq_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fq_add_l1ms2n_unit_test");
}

void Fq_add_l1ms2m_unit_test()
{
    //Fq_add_l1ms2m_test 0:
    FqElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_l1ms2m_test 1:
    FqElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_l1ms2m_test 2:
    FqElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fq_add_l1ms2m_test 3:
    FqElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms2m3= {0x0,0xc0000000,{0xc3df73e9278302b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fq_add(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fq_add(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fq_add(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fq_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fq_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fq_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fq_add_l1ms2m_unit_test");
}

void Fq_add_s1ml2m_unit_test()
{
    //Fq_add_s1ml2m_test 0:
    FqElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_s1ml2m_test 1:
    FqElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_s1ml2m_test 2:
    FqElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1ml2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fq_add_s1ml2m_test 3:
    FqElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1ml2m3= {0x0,0xc0000000,{0xc3df73e9278302b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fq_add(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fq_add(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fq_add(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fq_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fq_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fq_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fq_add_s1ml2m_unit_test");
}

void Fq_add_l1ns2_unit_test()
{
    //Fq_add_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x0,0x80000000,{0xa1f0fac9f8001bb8,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_add_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x80000000,{0xc3df73e9278402b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_add(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_add(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_add(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_add_l1ns2_unit_test");
}

void Fq_add_s1l2n_unit_test()
{
    //Fq_add_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fq_add_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_add_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x80000000,{0x1bb8e645ae220f97,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    //Fq_add_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x80000000,{0xc3df73e9278402b7,0x687e956e978e3572,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_add(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_add(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_add(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_add(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_add_s1l2n_unit_test");
}

void Fr_add_l1nl2n_unit_test()
{
    //Fr_add_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0x80000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fr_add_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x80000000,{0xbc1e0a6c0ffffffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_add(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_add(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_add(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_add_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_add_l1nl2n_unit_test");
}

void Fr_add_l1ml2n_unit_test()
{
    //Fr_add_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0xc0000000,{0x592c68389ffffff7,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_add_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0xc0000000,{0x592c68389ffffff6,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_add_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0xc0000000,{0x85d3abacbf0040,0xbe6290e259d86f01,0x6564e93fe2607c1b,0x252a8084f0653591}};
    //Fr_add_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0xc0000000,{0x6f22b2295e216dab,0x1d01c41c43fb8cb9,0x25dadfcddb423a57,0xf40c58efe5466576}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_add(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_add(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_add(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_add_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_add_l1ml2n_unit_test");
}

void Fr_add_l1ml2m_unit_test()
{
    //Fr_add_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fr_add_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0xc0000000,{0xbc1e0a6c0ffffffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_add(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_add(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_add(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_add_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_add_l1ml2m_unit_test");
}

void Fr_add_l1nl2m_unit_test()
{
    //Fr_add_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0xc0000000,{0xac96341c4ffffffd,0x36fc76959f60cd29,0x666ea36f7879462e,0xe0a77c19a07df2f}};
    //Fr_add_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0xc0000000,{0xe75ec7017e216daa,0xcc99f38b5088ab96,0xb53a5460d83f899c,0x1343bc0a22e32522}};
    //Fr_add_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0xc0000000,{0x6f22b2295e216dab,0x1d01c41c43fb8cb9,0x25dadfcddb423a57,0xf40c58efe5466576}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_add(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_add(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_add(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_add_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_add_l1nl2m_unit_test");
}

void Fr_add_s1nl2m_unit_test()
{
    //Fr_add_s1nl2m_test 0:
    FrElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m0= {0x0,0xc0000000,{0xac96341c4ffffffd,0x36fc76959f60cd29,0x666ea36f7879462e,0xe0a77c19a07df2f}};
    //Fr_add_s1nl2m_test 1:
    FrElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1nl2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_s1nl2m_test 2:
    FrElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1nl2m2= {0x0,0xc0000000,{0xf2c13c606e1e14fb,0xc06bd2fc0a5efd4d,0x4d406b95d15ca380,0x1861fa84426b2851}};
    //Fr_add_s1nl2m_test 3:
    FrElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_s1nl2m3= {0x0,0xc0000000,{0xd369cbe3b0000004,0x4903896a609f32d5,0x19915c908786b9d1,0x71f5883e65f820d0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fr_add(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fr_add(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fr_add(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fr_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fr_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fr_add_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fr_add_s1nl2m_unit_test");
}

void Fr_add_l1ms2n_unit_test()
{
    //Fr_add_l1ms2n_test 0:
    FrElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n0= {0x0,0xc0000000,{0x592c68389ffffff7,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_add_l1ms2n_test 1:
    FrElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2n1= {0x0,0xc0000000,{0x592c68389ffffff6,0x6df8ed2b3ec19a53,0xccdd46def0f28c5c,0x1c14ef83340fbe5e}};
    //Fr_add_l1ms2n_test 2:
    FrElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2n2= {0x0,0xc0000000,{0x8b033eb827ff6d5d,0x6093b892f9973313,0xb949a9cb070047f0,0x1396afa5c0a80a66}};
    //Fr_add_l1ms2n_test 3:
    FrElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FrElement pResult_l1ms2n3= {0x0,0xc0000000,{0xd369cbe3b0000004,0x4903896a609f32d5,0x19915c908786b9d1,0x71f5883e65f820d0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fr_add(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fr_add(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fr_add(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fr_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fr_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fr_add_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fr_add_l1ms2n_unit_test");
}

void Fr_add_l1ms2m_unit_test()
{
    //Fr_add_l1ms2m_test 0:
    FrElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_l1ms2m_test 1:
    FrElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_l1ms2m_test 2:
    FrElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fr_add_l1ms2m_test 3:
    FrElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms2m3= {0x0,0xc0000000,{0xbc1e0a6c0ffffffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fr_add(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fr_add(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fr_add(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fr_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fr_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fr_add_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fr_add_l1ms2m_unit_test");
}

void Fr_add_s1ml2m_unit_test()
{
    //Fr_add_s1ml2m_test 0:
    FrElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m0= {0x0,0xc0000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_s1ml2m_test 1:
    FrElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1ml2m1= {0x0,0xc0000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_s1ml2m_test 2:
    FrElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1ml2m2= {0x0,0xc0000000,{0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba}};
    //Fr_add_s1ml2m_test 3:
    FrElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1ml2m3= {0x0,0xc0000000,{0xbc1e0a6c0ffffffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fr_add(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fr_add(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fr_add(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fr_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fr_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fr_add_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fr_add_s1ml2m_unit_test");
}

void Fr_add_l1ns2_unit_test()
{
    //Fr_add_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x0,0x80000000,{0xa1f0fac9f8001bb8,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_add_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x80000000,{0xbc1e0a6c1000fffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_add(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_add(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_add(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_add_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_add_l1ns2_unit_test");
}

void Fr_add_s1l2n_unit_test()
{
    //Fr_add_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x80000000,{0x3,0x0,0x0,0x0}};
    //Fr_add_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_add_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x80000000,{0x1bb8e645ae220f97,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    //Fr_add_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x80000000,{0xbc1e0a6c1000fffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xcf9bb18d1ece5fd6}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_add(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_add(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_add(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_add(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_add_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_add_s1l2n_unit_test");
}

void Fr_toInt_unit_test()
{
    //Fr_toInt_test 0:
    FrElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrRawElement pRawResult0= {0xa1f0};
    //Fr_toInt_test 1:
    FrElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrRawElement pRawResult1= {0xa1f0};
    //Fr_toInt_test 2:
    FrElement pA2= {0x0,0x80000000,{0xa1f0,0x0,0x0,0x0}};
    FrRawElement pRawResult2= {0xa1f0};

    FrRawElement pRawResult0_c = {0};
    FrRawElement pRawResult1_c = {0};
    FrRawElement pRawResult2_c = {0};

    pRawResult0_c[0] = Fr_toInt(&pA0);
    pRawResult1_c[0] = Fr_toInt(&pA1);
    pRawResult2_c[0] = Fr_toInt(&pA2);

    compare_Result(pRawResult0, pRawResult0_c,&pA0,&pA0, 0, "Fr_toInt_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,&pA1,&pA1, 1, "Fr_toInt_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,&pA2,&pA2, 2, "Fr_toInt_unit_test");
    //compare_rawResult(pRawResult3, pRawResult3_c,pA2,pA2, 3, "Fr_toInt_unit_test");
}


void Fq_toInt_unit_test()
{
    //Fq_toInt_test 0:
    FqElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqRawElement pRawResult0= {0xa1f0};
    //Fq_toInt_test 1:
    FqElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqRawElement pRawResult1= {0xa1f0};
    //Fq_toInt_test 2:
    FqElement pA2= {0x0,0x80000000,{0xa1f0,0x0,0x0,0x0}};
    FqRawElement pRawResult2= {0xa1f0};

    FqRawElement pRawResult0_c = {0};
    FqRawElement pRawResult1_c = {0};
    FqRawElement pRawResult2_c = {0};

    pRawResult0_c[0] = Fq_toInt(&pA0);
    pRawResult1_c[0] = Fq_toInt(&pA1);
    pRawResult2_c[0] = Fq_toInt(&pA2);

    compare_Result(pRawResult0, pRawResult0_c,&pA0,&pA0, 0, "Fq_toInt_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,&pA1,&pA1, 1, "Fq_toInt_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,&pA2,&pA2, 2, "Fq_toInt_unit_test");
    //compare_rawResult(pRawResult3, pRawResult3_c,pA2,pA2, 3, "Fq_toInt_unit_test");
}

void Fr_lt_s1s2_unit_test()
{
    //Fr_lt_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_lt(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_lt(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_lt(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_lt_s1s2_unit_test");
}

void Fr_lt_l1nl2n_unit_test()
{
    //Fr_lt_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_lt(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_lt(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_lt(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_lt_l1nl2n_unit_test");
}

void Fr_lt_l1ml2n_unit_test()
{
    //Fr_lt_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_lt(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_lt(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_lt(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_lt_l1ml2n_unit_test");
}

void Fr_lt_l1ml2m_unit_test()
{
    //Fr_lt_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_lt(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_lt(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_lt(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_lt_l1ml2m_unit_test");
}

void Fr_lt_l1nl2m_unit_test()
{
    //Fr_lt_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_lt(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_lt(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_lt(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_lt_l1nl2m_unit_test");
}

// 6
void Fr_lt_s1l2m_unit_test()
{
    //Fr_lt_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_lt(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_lt(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_lt(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_lt_s1l2m_unit_test");
}

void Fr_lt_l1ms2_unit_test()
{
    //Fr_lt_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_lt(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_lt(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_lt(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_lt_l1ms2_unit_test");
}

void Fr_lt_l1ns2_unit_test()
{
    //Fr_lt_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_lt(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_lt(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_lt(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_lt_l1ns2_unit_test");
}

void Fr_lt_s1l2n_unit_test()
{
    //Fr_lt_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lt_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lt(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_lt(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_lt(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_lt(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_lt_s1l2n_unit_test");
}


void Fq_lt_s1s2_unit_test()
{
    //Fq_lt_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_lt(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_lt(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_lt(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_lt_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_lt_s1s2_unit_test");
}

void Fq_lt_l1nl2n_unit_test()
{
    //Fq_lt_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_lt(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_lt(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_lt(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_lt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_lt_l1nl2n_unit_test");
}

void Fq_lt_l1ml2n_unit_test()
{
    //Fq_lt_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_lt(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_lt(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_lt(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_lt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_lt_l1ml2n_unit_test");
}

void Fq_lt_l1ml2m_unit_test()
{
    //Fq_lt_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_lt(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_lt(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_lt(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_lt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_lt_l1ml2m_unit_test");
}

void Fq_lt_l1nl2m_unit_test()
{
    //Fq_lt_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_lt(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_lt(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_lt(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_lt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_lt_l1nl2m_unit_test");
}

// 6
void Fq_lt_s1l2m_unit_test()
{
    //Fq_lt_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_lt(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_lt(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_lt(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_lt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_lt_s1l2m_unit_test");
}

void Fq_lt_l1ms2_unit_test()
{
    //Fq_lt_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_lt(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_lt(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_lt(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_lt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_lt_l1ms2_unit_test");
}

void Fq_lt_l1ns2_unit_test()
{
    //Fq_lt_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_lt(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_lt(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_lt(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_lt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_lt_l1ns2_unit_test");
}

void Fq_lt_s1l2n_unit_test()
{
    //Fq_lt_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lt_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lt(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_lt(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_lt(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_lt(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_lt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_lt_s1l2n_unit_test");
}

void Fr_geq_s1s2_unit_test()
{
    //Fr_geq_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_geq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_geq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_geq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_geq_s1s2_unit_test");
}


void Fq_geq_s1s2_unit_test()
{
    //Fq_geq_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_geq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_geq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_geq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_geq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_geq_s1s2_unit_test");
}

void Fq_geq_l1nl2n_unit_test()
{
    //Fq_geq_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_geq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_geq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_geq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_geq_l1nl2n_unit_test");
}

void Fq_geq_l1ml2n_unit_test()
{
    //Fq_geq_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_geq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_geq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_geq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_geq_l1ml2n_unit_test");
}

void Fq_geq_l1ml2m_unit_test()
{
    //Fq_geq_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_geq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_geq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_geq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_geq_l1ml2m_unit_test");
}

void Fq_geq_l1nl2m_unit_test()
{
    //Fq_geq_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_geq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_geq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_geq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_geq_l1nl2m_unit_test");
}

void Fq_geq_s1l2m_unit_test()
{
    //Fq_geq_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_geq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_geq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_geq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_geq_s1l2m_unit_test");
}

void Fq_geq_l1ms2_unit_test()
{
    //Fq_geq_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_geq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_geq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_geq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c, &pA_l1ms23, &pB_l1ms23,3, "Fq_geq_l1ms2_unit_test");
}

void Fq_geq_l1ns2_unit_test()
{
    //Fq_geq_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_geq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_geq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_geq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c, &pA_l1ns20, &pB_l1ns20, 0, "Fq_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_geq_l1ns2_unit_test");
}

void Fq_geq_s1l2n_unit_test()
{
    //Fq_geq_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_geq_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_geq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_geq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_geq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_geq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_geq_s1l2n_unit_test");
}

void Fr_geq_l1nl2n_unit_test()
{
    //Fr_geq_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_geq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_geq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_geq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_geq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_geq_l1nl2n_unit_test");
}

void Fr_geq_l1ml2n_unit_test()
{
    //Fr_geq_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_geq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_geq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_geq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_geq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_geq_l1ml2n_unit_test");
}

void Fr_geq_l1ml2m_unit_test()
{
    //Fr_geq_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_geq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_geq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_geq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_geq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_geq_l1ml2m_unit_test");
}

void Fr_geq_l1nl2m_unit_test()
{
    //Fr_geq_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_geq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_geq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_geq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_geq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_geq_l1nl2m_unit_test");
}

void Fr_geq_s1l2m_unit_test()
{
    //Fr_geq_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_geq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_geq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_geq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_geq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_geq_s1l2m_unit_test");
}

void Fr_geq_l1ms2_unit_test()
{
    //Fr_geq_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_geq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_geq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_geq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_geq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c, &pA_l1ms23, &pB_l1ms23,3, "Fr_geq_l1ms2_unit_test");
}

void Fr_geq_l1ns2_unit_test()
{
    //Fr_geq_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_geq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_geq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_geq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c, &pA_l1ns20, &pB_l1ns20, 0, "Fr_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_geq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_geq_l1ns2_unit_test");
}

void Fr_geq_s1l2n_unit_test()
{
    //Fr_geq_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_geq_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_geq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_geq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_geq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_geq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_geq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_geq_s1l2n_unit_test");
}

void Fr_neg_unit_test()
{
    //Fr_neg_test 0:
    FrElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //FrElement pResult0= {0xffff5e10,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pResult0= {-41456,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neg_test 1:
    FrElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult1= {-41456,0x0,{0x0,0x0,0x0,0x0}};
    //FrElement pResult1= {0xffff5e10,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neg_test 2:
    FrElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000001,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fr_neg_test 3:
    FrElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000001,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};

    //Fr_neg_test 4:
    FrElement pA4= {INT_MIN,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult4= {0x0,0x80000000,{0x80000000,0x0,0x0,0x0}};

    //Fr_neg_test 5:
    FrElement pA5= {INT_MAX,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult5= {INT_MIN+1, 0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};
    FrElement Result4_c= {0,0,{0,0,0,0}};
    FrElement Result5_c= {0,0,{0,0,0,0}};

    Fr_neg(&Result0_c, &pA0);
    Fr_neg(&Result1_c, &pA1);
    Fr_neg(&Result2_c, &pA2);
    Fr_neg(&Result3_c, &pA3);
    Fr_neg(&Result4_c, &pA4);
    Fr_neg(&Result5_c, &pA5);

    compare_Result(&pResult0, &Result0_c,&pA0,&pA0, 0, "Fr_neg_unit_test");
    compare_Result(&pResult1, &Result1_c,&pA1,&pA1, 1, "Fr_neg_unit_test");
    compare_Result(&pResult2, &Result2_c,&pA2,&pA2, 2, "Fr_neg_unit_test");
    compare_Result(&pResult3, &Result3_c,&pA3,&pA3, 3, "Fr_neg_unit_test");
    compare_Result(&pResult4, &Result4_c,&pA4,&pA4, 4, "Fr_neg_unit_test");
    compare_Result(&pResult5, &Result5_c,&pA5,&pA5, 5, "Fr_neg_unit_test");
}


void Fq_neg_unit_test()
{
    //Fq_neg_test 0:
    FqElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //FqElement pResult0= {0xffff5e10,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pResult0= {-41456,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neg_test 1:
    FqElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult1= {-41456,0x0,{0x0,0x0,0x0,0x0}};
    //FqElement pResult1= {0xffff5e10,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neg_test 2:
    FqElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult2= {0xa1f0,0x80000000,{0x9a2f914ce07cfd47,0x0367766d2b951244,0xdc2822db40c0ac2f,0x183227397098d014}};
    //Fq_neg_test 3:
    FqElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult3= {0xa1f0,0xc0000000,{0x9a2f914ce07cfd47,0x0367766d2b951244,0xdc2822db40c0ac2f,0x183227397098d014}};

    //Fq_neg_test 4:
    FqElement pA4= {INT_MIN,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult4= {0x0,0x80000000,{0x80000000,0x0,0x0,0x0}};

    //Fq_neg_test 5:
    FqElement pA5= {INT_MAX,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult5= {INT_MIN+1, 0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};
    FqElement Result4_c= {0,0,{0,0,0,0}};
    FqElement Result5_c= {0,0,{0,0,0,0}};

    Fq_neg(&Result0_c, &pA0);
    Fq_neg(&Result1_c, &pA1);
    Fq_neg(&Result2_c, &pA2);
    Fq_neg(&Result3_c, &pA3);
    Fq_neg(&Result4_c, &pA4);
    Fq_neg(&Result5_c, &pA5);

    compare_Result(&pResult0, &Result0_c,&pA0,&pA0, 0, "Fq_neg_unit_test");
    compare_Result(&pResult1, &Result1_c,&pA1,&pA1, 1, "Fq_neg_unit_test");
    compare_Result(&pResult2, &Result2_c,&pA2,&pA2, 2, "Fq_neg_unit_test");
    compare_Result(&pResult3, &Result3_c,&pA3,&pA3, 3, "Fq_neg_unit_test");
    compare_Result(&pResult4, &Result4_c,&pA4,&pA4, 4, "Fq_neg_unit_test");
    compare_Result(&pResult5, &Result5_c,&pA5,&pA5, 5, "Fq_neg_unit_test");
}


void Fr_eq_s1s2_unit_test()
{
    //Fr_eq_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_eq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_eq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_eq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_eq_s1s2_unit_test");
}

void Fr_eq_l1nl2n_unit_test()
{
    //Fr_eq_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_eq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_eq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_eq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_eq_l1nl2n_unit_test");
}

void Fr_eq_l1ml2n_unit_test()
{
    //Fr_eq_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_eq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_eq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_eq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_eq_l1ml2n_unit_test");
}

void Fr_eq_l1ml2m_unit_test()
{
    //Fr_eq_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_eq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_eq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_eq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_eq_l1ml2m_unit_test");
}

void Fr_eq_l1nl2m_unit_test()
{
    //Fr_eq_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_eq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_eq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_eq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_eq_l1nl2m_unit_test");
}

void Fr_eq_s1l2m_unit_test()
{
    //Fr_eq_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_eq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_eq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_eq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c, &pA_s1l2m2, &pB_s1l2m2, 2, "Fr_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_eq_s1l2m_unit_test");
}

void Fr_eq_l1ms2_unit_test()
{
    //Fr_eq_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_eq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_eq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_eq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_eq_l1ms2_unit_test");
}

void Fr_eq_l1ns2_unit_test()
{
    //Fr_eq_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_eq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_eq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_eq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_eq_l1ns2_unit_test");
}

void Fr_eq_s1l2n_unit_test()
{
    //Fr_eq_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_eq_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_eq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_eq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_eq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_eq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_eq_s1l2n_unit_test");
}


void Fq_eq_s1s2_unit_test()
{
    //Fq_eq_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_eq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_eq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_eq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_eq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_eq_s1s2_unit_test");
}

void Fq_eq_l1nl2n_unit_test()
{
    //Fq_eq_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_eq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_eq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_eq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_eq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_eq_l1nl2n_unit_test");
}

void Fq_eq_l1ml2n_unit_test()
{
    //Fq_eq_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_eq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_eq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_eq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_eq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_eq_l1ml2n_unit_test");
}

void Fq_eq_l1ml2m_unit_test()
{
    //Fq_eq_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_eq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_eq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_eq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_eq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_eq_l1ml2m_unit_test");
}

void Fq_eq_l1nl2m_unit_test()
{
    //Fq_eq_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_eq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_eq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_eq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_eq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_eq_l1nl2m_unit_test");
}

void Fq_eq_s1l2m_unit_test()
{
    //Fq_eq_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_eq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_eq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_eq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c, &pA_s1l2m2, &pB_s1l2m2, 2, "Fq_eq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_eq_s1l2m_unit_test");
}

void Fq_eq_l1ms2_unit_test()
{
    //Fq_eq_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_eq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_eq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_eq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_eq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_eq_l1ms2_unit_test");
}

void Fq_eq_l1ns2_unit_test()
{
    //Fq_eq_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_eq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_eq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_eq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_eq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_eq_l1ns2_unit_test");
}

void Fq_eq_s1l2n_unit_test()
{
    //Fq_eq_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_eq_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_eq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_eq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_eq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_eq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_eq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_eq_s1l2n_unit_test");
}

void Fr_neq_s1s2_unit_test()
{
    //Fr_neq_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_neq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_neq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_neq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_neq_s1s2_unit_test");
}

void Fr_neq_l1nl2n_unit_test()
{
    //Fr_neq_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_neq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_neq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_neq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_neq_l1nl2n_unit_test");
}

void Fr_neq_l1ml2n_unit_test()
{
    //Fr_neq_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_neq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_neq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_neq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_neq_l1ml2n_unit_test");
}

void Fr_neq_l1ml2m_unit_test()
{
    //Fr_neq_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_neq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_neq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_neq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_neq_l1ml2m_unit_test");
}

void Fr_neq_l1nl2m_unit_test()
{
    //Fr_neq_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_neq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_neq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_neq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_neq_l1nl2m_unit_test");
}

// 6
void Fr_neq_s1l2m_unit_test()
{
    //Fr_neq_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_neq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_neq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_neq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_neq_s1l2m_unit_test");
}

void Fr_neq_l1ms2_unit_test()
{
    //Fr_neq_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_neq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_neq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_neq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_neq_l1ms2_unit_test");
}

void Fr_neq_l1ns2_unit_test()
{
    //Fr_neq_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_neq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_neq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_neq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_neq_l1ns2_unit_test");
}

void Fr_neq_s1l2n_unit_test()
{
    //Fr_neq_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_neq_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_neq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_neq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_neq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_neq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_neq_s1l2n_unit_test");
}


void Fq_neq_s1s2_unit_test()
{
    //Fq_neq_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_neq(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_neq(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_neq(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_neq_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_neq_s1s2_unit_test");
}

void Fq_neq_l1nl2n_unit_test()
{
    //Fq_neq_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_neq(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_neq(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_neq(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_neq_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_neq_l1nl2n_unit_test");
}

void Fq_neq_l1ml2n_unit_test()
{
    //Fq_neq_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_neq(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_neq(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_neq(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_neq_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_neq_l1ml2n_unit_test");
}

void Fq_neq_l1ml2m_unit_test()
{
    //Fq_neq_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_neq(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_neq(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_neq(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_neq_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_neq_l1ml2m_unit_test");
}

void Fq_neq_l1nl2m_unit_test()
{
    //Fq_neq_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_neq(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_neq(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_neq(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_neq_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_neq_l1nl2m_unit_test");
}

// 6
void Fq_neq_s1l2m_unit_test()
{
    //Fq_neq_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_neq(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_neq(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_neq(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_neq_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_neq_s1l2m_unit_test");
}

void Fq_neq_l1ms2_unit_test()
{
    //Fq_neq_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_neq(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_neq(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_neq(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_neq_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_neq_l1ms2_unit_test");
}

void Fq_neq_l1ns2_unit_test()
{
    //Fq_neq_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_neq(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_neq(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_neq(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_neq_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_neq_l1ns2_unit_test");
}

void Fq_neq_s1l2n_unit_test()
{
    //Fq_neq_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_neq_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_neq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_neq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_neq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_neq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_neq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_neq_s1l2n_unit_test");
}

void Fr_gt_s1s2_unit_test()
{
    //Fr_gt_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_gt(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_gt(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_gt(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_gt_s1s2_unit_test");
}

void Fr_gt_l1nl2n_unit_test()
{
    //Fr_gt_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_gt(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_gt(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_gt(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_gt_l1nl2n_unit_test");
}

void Fr_gt_l1ml2n_unit_test()
{
    //Fr_gt_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_gt(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_gt(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_gt(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_gt_l1ml2n_unit_test");
}

void Fr_gt_l1ml2m_unit_test()
{
    //Fr_gt_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_gt(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_gt(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_gt(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_gt_l1ml2m_unit_test");
}

void Fr_gt_l1nl2m_unit_test()
{
    //Fr_gt_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_gt(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_gt(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_gt(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0,0, "Fr_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1,1, "Fr_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2,2, "Fr_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3,3, "Fr_gt_l1nl2m_unit_test");
}

void Fr_gt_s1l2m_unit_test()
{
    //Fr_gt_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_gt(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_gt(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_gt(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_gt_s1l2m_unit_test");
}

void Fr_gt_l1ms2_unit_test()
{
    //Fr_gt_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x0,0x0,{0x0,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_gt(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_gt(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_gt(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_gt_l1ms2_unit_test");
}

void Fr_gt_l1ns2_unit_test()
{
    //Fr_gt_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_gt(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_gt(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_gt(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_gt_l1ns2_unit_test");
}

void Fr_gt_s1l2n_unit_test()
{
    //Fr_gt_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_gt_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_gt(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_gt(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_gt(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_gt(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_gt_s1l2n_unit_test");
}


void Fq_gt_s1s2_unit_test()
{
    //Fq_gt_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_gt(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_gt(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_gt(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_gt_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_gt_s1s2_unit_test");
}

void Fq_gt_l1nl2n_unit_test()
{
    //Fq_gt_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_gt(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_gt(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_gt(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_gt_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_gt_l1nl2n_unit_test");
}

void Fq_gt_l1ml2n_unit_test()
{
    //Fq_gt_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_gt(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_gt(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_gt(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_gt_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_gt_l1ml2n_unit_test");
}

void Fq_gt_l1ml2m_unit_test()
{
    //Fq_gt_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_gt(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_gt(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_gt(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_gt_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_gt_l1ml2m_unit_test");
}

void Fq_gt_l1nl2m_unit_test()
{
    //Fq_gt_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_gt(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_gt(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_gt(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0,0, "Fq_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1,1, "Fq_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2,2, "Fq_gt_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3,3, "Fq_gt_l1nl2m_unit_test");
}

void Fq_gt_s1l2m_unit_test()
{
    //Fq_gt_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_gt(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_gt(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_gt(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_gt_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_gt_s1l2m_unit_test");
}

void Fq_gt_l1ms2_unit_test()
{
    //Fq_gt_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_gt(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_gt(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_gt(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_gt_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_gt_l1ms2_unit_test");
}

void Fq_gt_l1ns2_unit_test()
{
    //Fq_gt_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_gt(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_gt(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_gt(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_gt_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_gt_l1ns2_unit_test");
}

void Fq_gt_s1l2n_unit_test()
{
    //Fq_gt_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_gt_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_gt(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_gt(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_gt(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_gt(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_gt_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_gt_s1l2n_unit_test");
}

void Fr_leq_s1l2n_unit_test()
{
    //Fr_leq_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_leq_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_leq_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_leq_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_leq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_leq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_leq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_leq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_leq_s1l2n_unit_test");
}


void Fq_leq_s1l2n_unit_test()
{
    //Fq_leq_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_leq_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_leq_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_leq_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_leq(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_leq(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_leq(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_leq(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_leq_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_leq_s1l2n_unit_test");
}


void Fr_band_s1s2_unit_test()
{
    //Fr_band_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1b0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x7fffffff,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_band(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_band(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_band(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_band_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_band_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_band_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_band_s1s2_unit_test");
}

void Fr_band_l1nl2n_unit_test()
{
    //Fr_band_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x0,0x80000000,{0x1b0e241a8000000,0x10183020205c1840,0x8c08021940808004,0x12003170084004}};
    //Fr_band_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x0,0x80000000,{0xbc1e0a6c0ffffffe,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0xf9bb18d1ece5fd6}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_band(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_band(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_band(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_band_l1nl2n_unit_test");
}

void Fr_band_l1ml2n_unit_test()
{
    //Fr_band_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_band_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x0,0x80000000,{0x11b0240128216102,0x3ac283181105841,0x409020402210084,0x650801f4e4481}};
    //Fr_band_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x0,0x80000000,{0x6786558e824ee6b4,0x1f24f29e98a78409,0xf02a37d1d2c8fb00,0x1a7855215e6c4b0c}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_band(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_band(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_band(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_band_l1ml2n_unit_test");
}

void Fr_band_l1ml2m_unit_test()
{
    //Fr_band_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x0,0x80000000,{0x981300004920100c,0xce101c001c807,0x800409c00c301818,0x1c3f00100800018}};
    //Fr_band_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x0,0x80000000,{0x49424100927735a,0x22ac641189204809,0x442c22442821002e,0x40a51c01a06d50b}};
    //Fr_band_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x0,0x80000000,{0x6786558e824ee6b4,0x1f24f29e98a78409,0xf02a37d1d2c8fb00,0x1a7855215e6c4b0c}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_band(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_band(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_band(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_band_l1ml2m_unit_test");
}

void Fr_band_l1nl2m_unit_test()
{
    //Fr_band_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x0,0x80000000,{0xa090300848000000,0x141874041c408808,0x4428224b4040042e,0x80227011000d004}};
    //Fr_band_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x0,0x80000000,{0x6786558e824ee6b4,0x1f24f29e98a78409,0xf02a37d1d2c8fb00,0x1a7855215e6c4b0c}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_band(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_band(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_band(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_band_l1nl2m_unit_test");
}

void Fr_band_s1l2m_unit_test()
{
    //Fr_band_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x0,0x80000000,{0xa1f0,0x0,0x0,0x0}};
    //Fr_band_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x0,0x80000000,{0xe6b4,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_band(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_band(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_band(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_band_s1l2m_unit_test");
}

void Fr_band_l1ms2_unit_test()
{
    //Fr_band_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fr_band_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x0,0x80000000,{0x1318,0x0,0x0,0x0}};
    //Fr_band_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x0,0x80000000,{0xe6b4,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_band(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_band(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_band(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_band_l1ms2_unit_test");
}

void Fr_band_l1ns2_unit_test()
{
    //Fr_band_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x80000000,{0xffff,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_band(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_band(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_band(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_band_l1ns2_unit_test");
}

void Fr_band_s1l2n_unit_test()
{
    //Fr_band_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fr_band_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x0,0x80000000,{0x21a0,0x0,0x0,0x0}};
    //Fr_band_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x0,0x80000000,{0xffff,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_band(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_band(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_band(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_band(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_band_s1l2n_unit_test");
}


void Fq_band_s1s2_unit_test()
{
    //Fq_band_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1b0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x7fffffff,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_band(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_band(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_band(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_band_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_band_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_band_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_band_s1s2_unit_test");
}

void Fq_band_l1nl2n_unit_test()
{
    //Fq_band_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0x80000000,{0x1b0e241a8000000,0x10183020205c1840,0x8c08021940808004,0x12003170084004}};
    //Fq_band_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0x80000000,{0xc3df73e9278302b8,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_band(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_band(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_band(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_band_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_band_l1nl2n_unit_test");
}

void Fq_band_l1ml2n_unit_test()
{
    //Fq_band_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_band_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0x80000000,{0x0b2042458c214000,0x433e30a0224408e3,0x08088205439b0004,0x000090010e4c4020}};
    //Fq_band_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0x80000000,{0x4e9c03ccd7320311,0xac61480c65f8dc94,0xe8ec5be6ca3cc583,0x01fd3901874bd9ef}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_band(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_band(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_band(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_band_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_band_l1ml2n_unit_test");
}

void Fq_band_l1ml2m_unit_test()
{
    //Fq_band_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0x80000000,{0x8c8080480008f227,0x2a20020000000160,0xc66389c8a5048050,0x2c6114615081c409}};
    //Fq_band_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0x80000000,{0x00041ac314688048,0x001a40a02a80086d,0x020c1406e0dc0406,0x2000100100300a28}};
    //Fq_band_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0x80000000,{0x4e9c03ccd7320311,0xac61480c65f8dc94,0xe8ec5be6ca3cc583,0x01fd3901874bd9ef}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_band(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_band(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_band(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_band_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_band_l1ml2m_unit_test");
}

void Fq_band_l1nl2m_unit_test()
{
    //Fq_band_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0x80000000,{0x1,0x0,0x0,0x0}};
    //Fq_band_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0x80000000,{0x80401ac950000000,0x0419402428880848,0x4428001a40c02406,0x0010203970901000}};
    //Fq_band_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0x80000000,{0x4e9c03ccd7320311,0xac61480c65f8dc94,0xe8ec5be6ca3cc583,0x01fd3901874bd9ef}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_band(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_band(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_band(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_band_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_band_l1nl2m_unit_test");
}

void Fq_band_s1l2m_unit_test()
{
    //Fq_band_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x0,0x80000000,{0x1,0x0,0x0,0x0}};
    //Fq_band_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x0,0x80000000,{0x0000000000008060,0x0,0x0,0x0}};
    //Fq_band_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x0,0x80000000,{0x0000000000000311,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_band(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_band(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_band(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_band_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_band_s1l2m_unit_test");
}

void Fq_band_l1ms2_unit_test()
{
    //Fq_band_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_band_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x0,0x80000000,{0x0000000000001008,0x0,0x0,0x0}};
    //Fq_band_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x0,0x80000000,{0x0000000000000311,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_band(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_band(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_band(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_band_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_band_l1ms2_unit_test");
}

void Fq_band_l1ns2_unit_test()
{
    //Fq_band_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x80000000,{0xffff,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_band(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_band(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_band(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_band_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_band_l1ns2_unit_test");
}

void Fq_band_s1l2n_unit_test()
{
    //Fq_band_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_band_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x0,0x80000000,{0x21a0,0x0,0x0,0x0}};
    //Fq_band_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x0,0x80000000,{0xffff,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_band(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_band(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_band(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_band(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_band_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_band_s1l2n_unit_test");
}

void Fr_land_s1s2_unit_test()
{
    //Fr_land_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_land(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_land(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_land(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_land_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_land_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_land_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_land_s1s2_unit_test");
}

void Fr_land_l1nl2n_unit_test()
{
    //Fr_land_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_land(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_land(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_land(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_land_l1nl2n_unit_test");
}

void Fr_land_l1ml2n_unit_test()
{
    //Fr_land_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_land(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_land(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_land(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_land_l1ml2n_unit_test");
}

void Fr_land_l1ml2m_unit_test()
{
    //Fr_land_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_land(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_land(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_land(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_land_l1ml2m_unit_test");
}

void Fr_land_l1nl2m_unit_test()
{
    //Fr_land_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_land(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_land(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_land(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_land_l1nl2m_unit_test");
}

// 6
void Fr_land_s1l2m_unit_test()
{
    //Fr_land_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_land(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_land(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_land(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_land_s1l2m_unit_test");
}

void Fr_land_l1ms2_unit_test()
{
    //Fr_land_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_land(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_land(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_land(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_land_l1ms2_unit_test");
}

void Fr_land_l1ns2_unit_test()
{
    //Fr_land_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_land(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_land(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_land(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_land_l1ns2_unit_test");
}

void Fr_land_s1l2n_unit_test()
{
    //Fr_land_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_land_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_land(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_land(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_land(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_land(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_land_s1l2n_unit_test");
}


void Fq_land_s1s2_unit_test()
{
    //Fq_land_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_land(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_land(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_land(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_land_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_land_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_land_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_land_s1s2_unit_test");
}

void Fq_land_l1nl2n_unit_test()
{
    //Fq_land_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_land(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_land(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_land(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_land_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_land_l1nl2n_unit_test");
}

void Fq_land_l1ml2n_unit_test()
{
    //Fq_land_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_land(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_land(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_land(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_land_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_land_l1ml2n_unit_test");
}

void Fq_land_l1ml2m_unit_test()
{
    //Fq_land_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_land(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_land(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_land(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_land_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_land_l1ml2m_unit_test");
}

void Fq_land_l1nl2m_unit_test()
{
    //Fq_land_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_land(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_land(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_land(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_land_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_land_l1nl2m_unit_test");
}

// 6
void Fq_land_s1l2m_unit_test()
{
    //Fq_land_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_land(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_land(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_land(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_land_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_land_s1l2m_unit_test");
}

void Fq_land_l1ms2_unit_test()
{
    //Fq_land_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_land(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_land(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_land(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_land_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_land_l1ms2_unit_test");
}

void Fq_land_l1ns2_unit_test()
{
    //Fq_land_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_land(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_land(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_land(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_land_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_land_l1ns2_unit_test");
}

void Fq_land_s1l2n_unit_test()
{
    //Fq_land_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_land_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_land(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_land(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_land(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_land(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_land_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_land_s1l2n_unit_test");
}

void Fr_lor_s1s2_unit_test()
{
    //Fr_lor_s1s2_test 0:
    FrElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1s2_test 1:
    FrElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1s2_test 2:
    FrElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1s2_test 3:
    FrElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fr_lor(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fr_lor(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fr_lor(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fr_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fr_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fr_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fr_lor_s1s2_unit_test");
}

void Fr_lor_l1nl2n_unit_test()
{
    //Fr_lor_l1nl2n_test 0:
    FrElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2n_test 1:
    FrElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2n_test 2:
    FrElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2n_test 3:
    FrElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fr_lor(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fr_lor(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fr_lor(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fr_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fr_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fr_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fr_lor_l1nl2n_unit_test");
}

void Fr_lor_l1ml2n_unit_test()
{
    //Fr_lor_l1ml2n_test 0:
    FrElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2n_test 1:
    FrElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2n_test 2:
    FrElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2n_test 3:
    FrElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fr_lor(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fr_lor(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fr_lor(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fr_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fr_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fr_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fr_lor_l1ml2n_unit_test");
}

void Fr_lor_l1ml2m_unit_test()
{
    //Fr_lor_l1ml2m_test 0:
    FrElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2m_test 1:
    FrElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2m_test 2:
    FrElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ml2m_test 3:
    FrElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fr_lor(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fr_lor(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fr_lor(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fr_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fr_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fr_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fr_lor_l1ml2m_unit_test");
}

void Fr_lor_l1nl2m_unit_test()
{
    //Fr_lor_l1nl2m_test 0:
    FrElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2m_test 1:
    FrElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2m_test 2:
    FrElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1nl2m_test 3:
    FrElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fr_lor(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fr_lor(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fr_lor(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fr_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fr_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fr_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fr_lor_l1nl2m_unit_test");
}

void Fr_lor_s1l2m_unit_test()
{
    //Fr_lor_s1l2m_test 0:
    FrElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2m_test 1:
    FrElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2m_test 2:
    FrElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2m_test 3:
    FrElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fr_lor(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fr_lor(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fr_lor(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fr_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fr_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fr_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fr_lor_s1l2m_unit_test");
}

void Fr_lor_l1ms2_unit_test()
{
    //Fr_lor_l1ms2_test 0:
    FrElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ms2_test 1:
    FrElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ms2_test 2:
    FrElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ms2_test 3:
    FrElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fr_lor(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fr_lor(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fr_lor(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fr_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fr_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fr_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fr_lor_l1ms2_unit_test");
}

void Fr_lor_l1ns2_unit_test()
{
    //Fr_lor_l1ns2_test 0:
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ns2_test 1:
    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ns2_test 2:
    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_l1ns2_test 3:
    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fr_lor(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fr_lor(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fr_lor(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fr_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fr_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fr_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fr_lor_l1ns2_unit_test");
}

void Fr_lor_s1l2n_unit_test()
{
    //Fr_lor_s1l2n_test 0:
    FrElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FrElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2n_test 1:
    FrElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FrElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FrElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2n_test 2:
    FrElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FrElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fr_lor_s1l2n_test 3:
    FrElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lor(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fr_lor(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fr_lor(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fr_lor(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fr_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fr_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fr_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fr_lor_s1l2n_unit_test");
}


void Fq_lor_s1s2_unit_test()
{
    //Fq_lor_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_lor(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_lor(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_lor(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_lor_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_lor_s1s2_unit_test");
}

void Fq_lor_l1nl2n_unit_test()
{
    //Fq_lor_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_lor(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_lor(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_lor(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_lor_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_lor_l1nl2n_unit_test");
}

void Fq_lor_l1ml2n_unit_test()
{
    //Fq_lor_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_lor(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_lor(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_lor(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_lor_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_lor_l1ml2n_unit_test");
}

void Fq_lor_l1ml2m_unit_test()
{
    //Fq_lor_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_lor(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_lor(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_lor(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_lor_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_lor_l1ml2m_unit_test");
}

void Fq_lor_l1nl2m_unit_test()
{
    //Fq_lor_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_lor(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_lor(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_lor(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_lor_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_lor_l1nl2m_unit_test");
}

void Fq_lor_s1l2m_unit_test()
{
    //Fq_lor_s1l2m_test 0:
    FqElement pA_s1l2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2m_test 1:
    FqElement pA_s1l2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2m1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2m_test 2:
    FqElement pA_s1l2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2m2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2m_test 3:
    FqElement pA_s1l2m3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2m3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_s1l2m0, &pB_s1l2m0);
    Fq_lor(&Result1_c, &pA_s1l2m1, &pB_s1l2m1);
    Fq_lor(&Result2_c, &pA_s1l2m2, &pB_s1l2m2);
    Fq_lor(&Result3_c, &pA_s1l2m3, &pB_s1l2m3);

    compare_Result(&pResult_s1l2m0, &Result0_c,&pA_s1l2m0, &pB_s1l2m0, 0, "Fq_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m1, &Result1_c,&pA_s1l2m1, &pB_s1l2m1, 1, "Fq_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m2, &Result2_c,&pA_s1l2m2, &pB_s1l2m2, 2, "Fq_lor_s1l2m_unit_test");
    compare_Result(&pResult_s1l2m3, &Result3_c,&pA_s1l2m3, &pB_s1l2m3, 3, "Fq_lor_s1l2m_unit_test");
}

void Fq_lor_l1ms2_unit_test()
{
    //Fq_lor_l1ms2_test 0:
    FqElement pA_l1ms20= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ms2_test 1:
    FqElement pA_l1ms21= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ms2_test 2:
    FqElement pA_l1ms22= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ms2_test 3:
    FqElement pA_l1ms23= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms23= {0x1,0x0,{0x0,0x0,0x0,0x0}};


    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1ms20, &pB_l1ms20);
    Fq_lor(&Result1_c, &pA_l1ms21, &pB_l1ms21);
    Fq_lor(&Result2_c, &pA_l1ms22, &pB_l1ms22);
    Fq_lor(&Result3_c, &pA_l1ms23, &pB_l1ms23);

    compare_Result(&pResult_l1ms20, &Result0_c,&pA_l1ms20, &pB_l1ms20, 0, "Fq_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms21, &Result1_c,&pA_l1ms21, &pB_l1ms21, 1, "Fq_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms22, &Result2_c,&pA_l1ms22, &pB_l1ms22, 2, "Fq_lor_l1ms2_unit_test");
    compare_Result(&pResult_l1ms23, &Result3_c,&pA_l1ms23, &pB_l1ms23, 3, "Fq_lor_l1ms2_unit_test");
}

void Fq_lor_l1ns2_unit_test()
{
    //Fq_lor_l1ns2_test 0:
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns20= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ns2_test 1:
    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns21= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ns2_test 2:
    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns22= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_l1ns2_test 3:
    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns23= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_l1ns20, &pB_l1ns20);
    Fq_lor(&Result1_c, &pA_l1ns21, &pB_l1ns21);
    Fq_lor(&Result2_c, &pA_l1ns22, &pB_l1ns22);
    Fq_lor(&Result3_c, &pA_l1ns23, &pB_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, &pB_l1ns20, 0, "Fq_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, &pB_l1ns21, 1, "Fq_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, &pB_l1ns22, 2, "Fq_lor_l1ns2_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, &pB_l1ns23, 3, "Fq_lor_l1ns2_unit_test");
}

void Fq_lor_s1l2n_unit_test()
{
    //Fq_lor_s1l2n_test 0:
    FqElement pA_s1l2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1l2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n0= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2n_test 1:
    FqElement pA_s1l2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1l2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1l2n1= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2n_test 2:
    FqElement pA_s1l2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1l2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1l2n2= {0x1,0x0,{0x0,0x0,0x0,0x0}};
    //Fq_lor_s1l2n_test 3:
    FqElement pA_s1l2n3= {0xffff,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1l2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1l2n3= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lor(&Result0_c, &pA_s1l2n0, &pB_s1l2n0);
    Fq_lor(&Result1_c, &pA_s1l2n1, &pB_s1l2n1);
    Fq_lor(&Result2_c, &pA_s1l2n2, &pB_s1l2n2);
    Fq_lor(&Result3_c, &pA_s1l2n3, &pB_s1l2n3);

    compare_Result(&pResult_s1l2n0, &Result0_c,&pA_s1l2n0, &pB_s1l2n0, 0, "Fq_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n1, &Result1_c,&pA_s1l2n1, &pB_s1l2n1, 1, "Fq_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n2, &Result2_c,&pA_s1l2n2, &pB_s1l2n2, 2, "Fq_lor_s1l2n_unit_test");
    compare_Result(&pResult_s1l2n3, &Result3_c,&pA_s1l2n3, &pB_s1l2n3, 3, "Fq_lor_s1l2n_unit_test");
}

void Fq_lnot_unit_test()
{
    FqElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FqElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_lnot(&Result0_c, &pA_l1ns20);
    Fq_lnot(&Result1_c, &pA_l1ns21);
    Fq_lnot(&Result2_c, &pA_l1ns22);
    Fq_lnot(&Result3_c, &pA_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, 0, "Fq_lnot_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, 1, "Fq_lnot_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, 2, "Fq_lnot_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, 3, "Fq_lnot_unit_test");
}

void Fr_lnot_unit_test()
{
    FrElement pA_l1ns20= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FrElement pResult_l1ns20= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement pA_l1ns21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FrElement pResult_l1ns21= {0x1,0x0,{0x0,0x0,0x0,0x0}};

    FrElement pA_l1ns22= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FrElement pResult_l1ns22= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement pA_l1ns23= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FrElement pResult_l1ns23= {0x0,0x0,{0x0,0x0,0x0,0x0}};

    FrElement Result0_c = {0,0,{0,0,0,0}};
    FrElement Result1_c = {0,0,{0,0,0,0}};
    FrElement Result2_c= {0,0,{0,0,0,0}};
    FrElement Result3_c= {0,0,{0,0,0,0}};

    Fr_lnot(&Result0_c, &pA_l1ns20);
    Fr_lnot(&Result1_c, &pA_l1ns21);
    Fr_lnot(&Result2_c, &pA_l1ns22);
    Fr_lnot(&Result3_c, &pA_l1ns23);

    compare_Result(&pResult_l1ns20, &Result0_c,&pA_l1ns20, 0, "Fr_lnot_unit_test");
    compare_Result(&pResult_l1ns21, &Result1_c,&pA_l1ns21, 1, "Fr_lnot_unit_test");
    compare_Result(&pResult_l1ns22, &Result2_c,&pA_l1ns22, 2, "Fr_lnot_unit_test");
    compare_Result(&pResult_l1ns23, &Result3_c,&pA_l1ns23, 3, "Fr_lnot_unit_test");
}

void Fr_shr_test(FrElement r_expected, FrElement a, FrElement b, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_shr(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fr_shr_short_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fr_shr_test(fr_short(r_expected), fr_short(a), fr_short(b), index);
}

void Fr_shr_mshort_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fr_shr_test(fr_mshort(r_expected), fr_mshort(a), fr_short(b), index);
}

void Fr_shr_unit_test()
{
    Fr_shr_short_test(        0,     0xa1f0, 0x1bb8,   0);
    Fr_shr_short_test(   0xa1f0,     0xa1f0,       0,  1);
    Fr_shr_short_test(   0x50f8,     0xa1f0,       1,  2);
    Fr_shr_short_test(  0x143e0,     0xa1f0,      -1,  3);
    Fr_shr_short_test(0x000287c,     0xa1f0,       2,  4);
    Fr_shr_short_test(0x00287c0,     0xa1f0,      -2,  5);
    Fr_shr_short_test(      0xa,     0xa1f0,      12,  6);
    Fr_shr_short_test(0xa1f0000,     0xa1f0,     -12,  7);
    Fr_shr_short_test(        7, 0x7000a1ff,      28,  8);
    Fr_shr_short_test(        0,     0xa1f0,      31,  9);
    Fr_shr_short_test(        0,     0xa1f0,      67, 10);
    Fr_shr_short_test(        0,     0xa1f0,     256, 11);


    FrElement a21 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b21 = fr_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a22 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b22 = fr_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a23 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b23 = fr_long(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a24 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b24 = fr_mlong(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a25 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b25 = fr_long(0x1bb8e645ae216da7);

    FrElement a26 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b26 = fr_mlong(0x1bb8e645ae216da7);

    Fr_shr_test(fr_short(0), a21, b21, 21);
    Fr_shr_test(fr_short(0), a22, b22, 22);
    Fr_shr_test(fr_short(0), a23, b23, 23);
    Fr_shr_test(fr_short(0), a24, b24, 24);
    Fr_shr_test(fr_short(0), a25, b25, 25);
    Fr_shr_test(fr_short(0), a26, b26, 26);

    FrElement r31 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement r32 = fr_long(0x50f87d64fc000000,0x4a0cfa121e6e5c24,0x6e14116da0605617,0x0c19139cb84c680a);
    FrElement r33 = fr_long(0x450f87d64fc00000,0x74a0cfa121e6e5c2,0xa6e14116da060561,0x00c19139cb84c680);
    FrElement r34 = fr_long(0x848a1f0fac9f8000,0xc2e9419f4243cdcb,0x014dc2822db40c0a,0x000183227397098d);
    FrElement r35 = fr_long(0x72e12287c3eb27e0,0x02b0ba5067d090f3,0x63405370a08b6d03,0x00000060c89ce5c2);
    FrElement r36 = fr_long(0x3cdcb848a1f0fac9,0x40c0ac2e9419f424,0x7098d014dc2822db,0x0000000018322739);
    FrElement r37 = fr_long(0x4dc2822db40c0ac2,0x0183227397098d01,0x0000000000000000,0x0000000000000000);
    FrElement r38 = fr_long(0x0000000000183227,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r41 = fr_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r42 = fr_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FrElement r43 = fr_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FrElement r44 = fr_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FrElement r45 = fr_long(0xb41e0a6c0fffffff,0x14a8d00028378a38,0x8870667812989bc7,0x003481a1faf682b1);
    FrElement r46 = fr_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);

    Fr_shr_test(r31,         a21, fr_short(0),    31);
    Fr_shr_test(r32,         a21, fr_short(1),    32);
    Fr_shr_test(r33,         a21, fr_short(5),    33);
    Fr_shr_test(r34,         a21, fr_short(12),   34);
    Fr_shr_test(r35,         a21, fr_short(22),   35);
    Fr_shr_test(r36,         a21, fr_short(32),   36);
    Fr_shr_test(r37,         a21, fr_short(132),  37);
    Fr_shr_test(r38,         a21, fr_short(232),  38);
    Fr_shr_test(fr_short(0), a21, fr_short(432),  39);

    Fr_shr_test(r41,         a21, fr_short(-1),   41);
    Fr_shr_test(r42,         a21, fr_short(-5),   42);
    Fr_shr_test(r43,         a21, fr_short(-12),  43);
    Fr_shr_test(r44,         a21, fr_short(-22),  44);
    Fr_shr_test(r45,         a21, fr_short(-32),  45);
    Fr_shr_test(r46,         a21, fr_short(-132), 46);
    Fr_shr_test(fr_long(0),  a21, fr_short(-232), 47);
    Fr_shr_test(fr_short(0), a21, fr_short(-332), 48);
    Fr_shr_test(fr_short(0), a21, fr_short(-432), 49);

    Fr_shr_test(r31,         a21, fr_long(0),    51);
    Fr_shr_test(r32,         a21, fr_long(1),    52);
    Fr_shr_test(r33,         a21, fr_long(5),    53);
    Fr_shr_test(r34,         a21, fr_long(12),   54);
    Fr_shr_test(r35,         a21, fr_long(22),   55);
    Fr_shr_test(r36,         a21, fr_long(32),   56);
    Fr_shr_test(r37,         a21, fr_long(132),  57);
    Fr_shr_test(r38,         a21, fr_long(232),  58);
    Fr_shr_test(fr_short(0), a21, fr_long(432),  59);

    Fr_shr_test(fr_short(0), a21, fr_long(-1),   61);
    Fr_shr_test(fr_short(0), a21, fr_long(-5),   62);
    Fr_shr_test(fr_short(0), a21, fr_long(-12),  63);
    Fr_shr_test(fr_short(0), a21, fr_long(-22),  64);
    Fr_shr_test(fr_short(0), a21, fr_long(-32),  65);
    Fr_shr_test(fr_short(0), a21, fr_long(-132), 66);
    Fr_shr_test(fr_short(0), a21, fr_long(-232), 67);
    Fr_shr_test(fr_short(0), a21, fr_long(-332), 68);
    Fr_shr_test(fr_short(0), a21, fr_long(-432), 69);

    Fr_shr_test(fr_short(0), a21, fr_mlong(1),    71);
    Fr_shr_test(fr_short(0), a21, fr_mlong(12),   72);
    Fr_shr_test(fr_short(0), a21, fr_mlong(32),   73);
    Fr_shr_test(fr_short(0), a21, fr_mlong(132),  74);
    Fr_shr_test(fr_short(0), a21, fr_mlong(432),  75);
    Fr_shr_test(fr_short(0), a21, fr_mlong(-1),   76);
    Fr_shr_test(fr_short(0), a21, fr_mlong(-5),   77);
    Fr_shr_test(fr_short(0), a21, fr_mlong(-12),  78);

    FrElement r80 = fr_long(0x55b425913927735a,0xa3ac6d7389307a4d,0x543d3ec42a2529ae,0x256e51ca1fcef59b);
    FrElement r81 = fr_long(0xaada12c89c93b9ad,0x51d636b9c4983d26,0xaa1e9f62151294d7,0x12b728e50fe77acd);
    FrElement r82 = fr_long(0xa4d55b4259139277,0x9aea3ac6d7389307,0x59b543d3ec42a252,0x000256e51ca1fcef);
    FrElement r83 = fr_long(0x89307a4d55b42591,0x2a2529aea3ac6d73,0x1fcef59b543d3ec4,0x00000000256e51ca);
    FrElement r84 = fr_long(0xb543d3ec42a2529a,0x0256e51ca1fcef59,0x0000000000000000,0x0000000000000000);
    FrElement r85 = fr_short(0);
    FrElement r86 = fr_long(0xab684b22724ee6b4,0x4758dae71260f49a,0xa87a7d88544a535d,0x0adca3943f9deb36);
    FrElement r87 = fr_long(0x3927735a00000000,0x89307a4d55b42591,0x2a2529aea3ac6d73,0x1fcef59b543d3ec4);
    FrElement r88 = fr_long(0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0xa2f2135d10f5dd42,0x0a6288c5b1d604ab);
    FrElement r89 = fr_short(0);

    Fr_shr_test(r80, a22, fr_short(0),    80);
    Fr_shr_test(r81, a22, fr_short(1),    81);
    Fr_shr_test(r82, a22, fr_short(12),   82);
    Fr_shr_test(r83, a22, fr_short(32),   83);
    Fr_shr_test(r84, a22, fr_short(132),  84);
    Fr_shr_test(r85, a22, fr_short(432),  85);
    Fr_shr_test(r86, a22, fr_short(-1),   86);
    Fr_shr_test(r87, a22, fr_short(-32),  87);
    Fr_shr_test(r88, a22, fr_short(-132), 88);
    Fr_shr_test(r89, a22, fr_short(-432), 89);
}

void Fr_shl_test(FrElement r_expected, FrElement a, FrElement b, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_shl(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fr_shl_short_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fr_shl_test(fr_short(r_expected), fr_short(a), fr_short(b), index);
}

void Fr_shl_mshort_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fr_shl_test(fr_mshort(r_expected), fr_mshort(a), fr_short(b), index);
}

void Fr_shl_unit_test()
{
    Fr_shl_short_test(        0,     0xa1f0, 0x1bb8,   0);
    Fr_shl_short_test(   0xa1f0,     0xa1f0,       0,  1);
    Fr_shl_short_test(0x000143e0,    0xa1f0,       1,  2);
    Fr_shl_short_test(0x000050f8,    0xa1f0,      -1,  3);
    Fr_shl_short_test(0x000287c0,    0xa1f0,       2,  4);
    Fr_shl_short_test(0x0000287c,    0xa1f0,      -2,  5);
    Fr_shl_short_test(0x0000050f,    0xa1f0,      -5,  6);
    Fr_shl_short_test(0x0a1f0000,    0xa1f0,      12,  7);
    Fr_shl_short_test(      0xa,     0xa1f0,     -12,  8);
    Fr_shl_short_test(        0,     0xa1f0,     -22,  9);
    Fr_shl_short_test(        0,     0xa1f0,     256, 10);


    FrElement a21 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b21 = fr_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a22 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b22 = fr_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a23 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b23 = fr_long(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a24 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b24 = fr_mlong(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FrElement a25 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b25 = fr_long(0x1bb8e645ae216da7);

    FrElement a26 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement b26 = fr_mlong(0x1bb8e645ae216da7);


    Fr_shl_test(fr_short(0), a21, b21, 21);
    Fr_shl_test(fr_short(0), a22, b22, 22);
    Fr_shl_test(fr_short(0), a23, b23, 23);
    Fr_shl_test(fr_short(0), a24, b24, 24);
    Fr_shl_test(fr_short(0), a25, b25, 25);
    Fr_shl_test(fr_short(0), a26, b26, 26);


    FrElement r31 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement r32 = fr_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r33 = fr_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FrElement r34 = fr_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FrElement r35 = fr_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FrElement r36 = fr_long(0xb41e0a6c0fffffff,0x14a8d00028378a38,0x8870667812989bc7,0x003481a1faf682b1);
    FrElement r37 = fr_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);
    FrElement r41 = fr_long(0x50f87d64fc000000,0x4a0cfa121e6e5c24,0x6e14116da0605617,0x0c19139cb84c680a);
    FrElement r42 = fr_long(0x450f87d64fc00000,0x74a0cfa121e6e5c2,0xa6e14116da060561,0x00c19139cb84c680);
    FrElement r43 = fr_long(0x848a1f0fac9f8000,0xc2e9419f4243cdcb,0x014dc2822db40c0a,0x000183227397098d);
    FrElement r44 = fr_long(0x72e12287c3eb27e0,0x02b0ba5067d090f3,0x63405370a08b6d03,0x00000060c89ce5c2);
    FrElement r45 = fr_long(0x3cdcb848a1f0fac9,0x40c0ac2e9419f424,0x7098d014dc2822db,0x0000000018322739);
    FrElement r46 = fr_long(0x4dc2822db40c0ac2,0x0183227397098d01,0x0000000000000000,0x0000000000000000);
    FrElement r47 = fr_long(0x0000000000183227,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fr_shl_test(r31,         a21, fr_short(0),    31);
    Fr_shl_test(r32,         a21, fr_short(1),    32);
    Fr_shl_test(r33,         a21, fr_short(5),    33);
    Fr_shl_test(r34,         a21, fr_short(12),   34);
    Fr_shl_test(r35,         a21, fr_short(22),   35);
    Fr_shl_test(r36,         a21, fr_short(32),   36);
    Fr_shl_test(r37,         a21, fr_short(132),  37);
    Fr_shl_test(fr_long(0),  a21, fr_short(232),  38);
    Fr_shl_test(fr_short(0), a21, fr_short(432),  39);

    Fr_shl_test(r41,         a21, fr_short(-1),   41);
    Fr_shl_test(r42,         a21, fr_short(-5),   42);
    Fr_shl_test(r43,         a21, fr_short(-12),  43);
    Fr_shl_test(r44,         a21, fr_short(-22),  44);
    Fr_shl_test(r45,         a21, fr_short(-32),  45);
    Fr_shl_test(r46,         a21, fr_short(-132), 46);
    Fr_shl_test(r47,         a21, fr_short(-232), 47);
    Fr_shl_test(fr_short(0), a21, fr_short(-332), 48);
    Fr_shl_test(fr_short(0), a21, fr_short(-432), 49);

    FrElement r51 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement r52 = fr_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r53 = fr_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FrElement r54 = fr_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FrElement r55 = fr_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FrElement r56 = fr_long(0xb41e0a6c0fffffff,0x14a8d00028378a38,0x8870667812989bc7,0x003481a1faf682b1);
    FrElement r57 = fr_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);

    Fr_shl_test(r51,         a21, fr_long(0),    51);
    Fr_shl_test(r52,         a21, fr_long(1),    52);
    Fr_shl_test(r53,         a21, fr_long(5),    53);
    Fr_shl_test(r54,         a21, fr_long(12),   54);
    Fr_shl_test(r55,         a21, fr_long(22),   55);
    Fr_shl_test(r56,         a21, fr_long(32),   56);
    Fr_shl_test(r57,         a21, fr_long(132),  57);
    Fr_shl_test(fr_long(0),  a21, fr_long(232),  58);
    Fr_shl_test(fr_short(0), a21, fr_long(432),  59);

    Fr_shl_test(fr_short(0), a21, fr_long(-1),   61);
    Fr_shl_test(fr_short(0), a21, fr_long(-5),   62);
    Fr_shl_test(fr_short(0), a21, fr_long(-12),  63);
    Fr_shl_test(fr_short(0), a21, fr_long(-22),  64);
    Fr_shl_test(fr_short(0), a21, fr_long(-32),  65);
    Fr_shl_test(fr_short(0), a21, fr_long(-132), 66);
    Fr_shl_test(fr_short(0), a21, fr_long(-232), 67);
    Fr_shl_test(fr_short(0), a21, fr_long(-332), 68);
    Fr_shl_test(fr_short(0), a21, fr_long(-432), 69);

    Fr_shl_test(fr_short(0), a21, fr_mlong(1),    71);
    Fr_shl_test(fr_short(0), a21, fr_mlong(12),   72);
    Fr_shl_test(fr_short(0), a21, fr_mlong(32),   73);
    Fr_shl_test(fr_short(0), a21, fr_mlong(132),  74);
    Fr_shl_test(fr_short(0), a21, fr_mlong(432),  75);
    Fr_shl_test(fr_short(0), a21, fr_mlong(-1),   76);
    Fr_shl_test(fr_short(0), a21, fr_mlong(-5),   77);
    Fr_shl_test(fr_short(0), a21, fr_mlong(-12),  78);

    FrElement r80 = fr_long(0x55b425913927735a,0xa3ac6d7389307a4d,0x543d3ec42a2529ae,0x256e51ca1fcef59b);
    FrElement r81 = fr_long(0xab684b22724ee6b4,0x4758dae71260f49a,0xa87a7d88544a535d,0x0adca3943f9deb36);
    FrElement r82 = fr_long(0x425913927735a000,0xc6d7389307a4d55b,0xd3ec42a2529aea3a,0x251ca1fcef59b543);
    FrElement r83 = fr_long(0x3927735a00000000,0x89307a4d55b42591,0x2a2529aea3ac6d73,0x1fcef59b543d3ec4);
    FrElement r84 = fr_long(0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0xa2f2135d10f5dd42,0x0a6288c5b1d604ab);
    FrElement r85 = fr_short(0);
    FrElement r86 = fr_long(0xaada12c89c93b9ad,0x51d636b9c4983d26,0xaa1e9f62151294d7,0x12b728e50fe77acd);
    FrElement r87 = fr_long(0x89307a4d55b42591,0x2a2529aea3ac6d73,0x1fcef59b543d3ec4,0x00000000256e51ca);
    FrElement r88 = fr_long(0xb543d3ec42a2529a,0x0256e51ca1fcef59,0x0000000000000000,0x0000000000000000);
    FrElement r89 = fr_short(0);

    Fr_shl_test(r80, a22, fr_short(0),    80);
    Fr_shl_test(r81, a22, fr_short(1),    81);
    Fr_shl_test(r82, a22, fr_short(12),   82);
    Fr_shl_test(r83, a22, fr_short(32),   83);
    Fr_shl_test(r84, a22, fr_short(132),  84);
    Fr_shl_test(r85, a22, fr_short(432),  85);
    Fr_shl_test(r86, a22, fr_short(-1),   86);
    Fr_shl_test(r87, a22, fr_short(-32),  87);
    Fr_shl_test(r88, a22, fr_short(-132), 88);
    Fr_shl_test(r89, a22, fr_short(-432), 89);
}


void Fq_shr_test(FqElement r_expected, FqElement a, FqElement b, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_shr(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fq_shr_short_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fq_shr_test(fq_short(r_expected), fq_short(a), fq_short(b), index);
}

void Fq_shr_mshort_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fq_shr_test(fq_mshort(r_expected), fq_mshort(a), fq_short(b), index);
}

void Fq_shr_unit_test()
{
    Fq_shr_short_test(        0,     0xa1f0, 0x1bb8,   0);
    Fq_shr_short_test(   0xa1f0,     0xa1f0,       0,  1);
    Fq_shr_short_test(   0x50f8,     0xa1f0,       1,  2);
    Fq_shr_short_test(  0x143e0,     0xa1f0,      -1,  3);
    Fq_shr_short_test(0x000287c,     0xa1f0,       2,  4);
    Fq_shr_short_test(0x00287c0,     0xa1f0,      -2,  5);
    Fq_shr_short_test(      0xa,     0xa1f0,      12,  6);
    Fq_shr_short_test(0xa1f0000,     0xa1f0,     -12,  7);
    Fq_shr_short_test(        7, 0x7000a1ff,      28,  8);
    Fq_shr_short_test(        0,     0xa1f0,      31,  9);
    Fq_shr_short_test(        0,     0xa1f0,      67, 10);
    Fq_shr_short_test(        0,     0xa1f0,     256, 11);


    FqElement a21 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b21 = fq_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a22 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b22 = fq_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a23 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b23 = fq_long(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a24 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b24 = fq_mlong(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a25 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b25 = fq_long(0x1bb8e645ae216da7);

    FqElement a26 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b26 = fq_mlong(0x1bb8e645ae216da7);

    Fq_shr_test(fq_short(0), a21, b21, 21);
    Fq_shr_test(fq_short(0), a22, b22, 22);
    Fq_shr_test(fq_short(0), a23, b23, 23);
    Fq_shr_test(fq_short(0), a24, b24, 24);
    Fq_shr_test(fq_short(0), a25, b25, 25);
    Fq_shr_test(fq_short(0), a26, b26, 26);

    FqElement r31 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement r32 = fq_long(0x50f87d64fc000000,0x4a0cfa121e6e5c24,0x6e14116da0605617,0x0c19139cb84c680a);
    FqElement r33 = fq_long(0x450f87d64fc00000,0x74a0cfa121e6e5c2,0xa6e14116da060561,0x00c19139cb84c680);
    FqElement r34 = fq_long(0x848a1f0fac9f8000,0xc2e9419f4243cdcb,0x014dc2822db40c0a,0x000183227397098d);
    FqElement r35 = fq_long(0x72e12287c3eb27e0,0x02b0ba5067d090f3,0x63405370a08b6d03,0x00000060c89ce5c2);
    FqElement r36 = fq_long(0x3cdcb848a1f0fac9,0x40c0ac2e9419f424,0x7098d014dc2822db,0x0000000018322739);
    FqElement r37 = fq_long(0x4dc2822db40c0ac2,0x0183227397098d01,0x0000000000000000,0x0000000000000000);
    FqElement r38 = fq_long(0x0000000000183227,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r41 = fq_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r42 = fq_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FqElement r43 = fq_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FqElement r44 = fq_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FqElement r45 = fq_long(0xbbdf73e9278302b9,0xa55b4db7397f303c,0x8870667812989bc6,0x003481a1faf682b1);
    FqElement r46 = fq_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);

    Fq_shr_test(r31,         a21, fq_short(0),    31);
    Fq_shr_test(r32,         a21, fq_short(1),    32);
    Fq_shr_test(r33,         a21, fq_short(5),    33);
    Fq_shr_test(r34,         a21, fq_short(12),   34);
    Fq_shr_test(r35,         a21, fq_short(22),   35);
    Fq_shr_test(r36,         a21, fq_short(32),   36);
    Fq_shr_test(r37,         a21, fq_short(132),  37);
    Fq_shr_test(r38,         a21, fq_short(232),  38);
    Fq_shr_test(fq_short(0), a21, fq_short(432),  39);

    Fq_shr_test(r41,         a21, fq_short(-1),   41);
    Fq_shr_test(r42,         a21, fq_short(-5),   42);
    Fq_shr_test(r43,         a21, fq_short(-12),  43);
    Fq_shr_test(r44,         a21, fq_short(-22),  44);
    Fq_shr_test(r45,         a21, fq_short(-32),  45);
    Fq_shr_test(r46,         a21, fq_short(-132), 46);
    Fq_shr_test(fq_long(0),  a21, fq_short(-232), 47);
    Fq_shr_test(fq_short(0), a21, fq_short(-332), 48);
    Fq_shr_test(fq_short(0), a21, fq_short(-432), 49);

    Fq_shr_test(r31,         a21, fq_long(0),    51);
    Fq_shr_test(r32,         a21, fq_long(1),    52);
    Fq_shr_test(r33,         a21, fq_long(5),    53);
    Fq_shr_test(r34,         a21, fq_long(12),   54);
    Fq_shr_test(r35,         a21, fq_long(22),   55);
    Fq_shr_test(r36,         a21, fq_long(32),   56);
    Fq_shr_test(r37,         a21, fq_long(132),  57);
    Fq_shr_test(r38,         a21, fq_long(232),  58);
    Fq_shr_test(fq_short(0), a21, fq_long(432),  59);

    Fq_shr_test(fq_short(0), a21, fq_long(-1),   61);
    Fq_shr_test(fq_short(0), a21, fq_long(-5),   62);
    Fq_shr_test(fq_short(0), a21, fq_long(-12),  63);
    Fq_shr_test(fq_short(0), a21, fq_long(-22),  64);
    Fq_shr_test(fq_short(0), a21, fq_long(-32),  65);
    Fq_shr_test(fq_short(0), a21, fq_long(-132), 66);
    Fq_shr_test(fq_short(0), a21, fq_long(-232), 67);
    Fq_shr_test(fq_short(0), a21, fq_long(-332), 68);
    Fq_shr_test(fq_short(0), a21, fq_long(-432), 69);

    Fq_shr_test(fq_short(0), a21, fq_mlong(1),    71);
    Fq_shr_test(fq_short(0), a21, fq_mlong(12),   72);
    Fq_shr_test(fq_short(0), a21, fq_mlong(32),   73);
    Fq_shr_test(fq_short(0), a21, fq_mlong(132),  74);
    Fq_shr_test(fq_short(0), a21, fq_mlong(432),  75);
    Fq_shr_test(fq_short(0), a21, fq_mlong(-1),   76);
    Fq_shr_test(fq_short(0), a21, fq_mlong(-5),   77);
    Fq_shr_test(fq_short(0), a21, fq_mlong(-12),  78);

    FqElement r80 = fq_long(0x0f245ae79cebd048,0x6b3ef4a83ac6acff,0x0a9c9ec7ebdf450e,0x240191410e7c4b2a);
    FqElement r81 = fq_long(0x87922d73ce75e824,0x359f7a541d63567f,0x054e4f63f5efa287,0x1200c8a0873e2595);
    FqElement r82 = fq_long(0xcff0f245ae79cebd,0x50e6b3ef4a83ac6a,0xb2a0a9c9ec7ebdf4,0x000240191410e7c4);
    FqElement r83 = fq_long(0x3ac6acff0f245ae7,0xebdf450e6b3ef4a8,0x0e7c4b2a0a9c9ec7,0x0000000024019141);
    FqElement r84 = fq_long(0xa0a9c9ec7ebdf450,0x0240191410e7c4b2,0x0000000000000000,0x0000000000000000);
    FqElement r85 = fq_short(0);
    FqElement r86 = fq_long(0x1e48b5cf39d7a090,0xd67de950758d59fe,0x15393d8fd7be8a1c,0x080322821cf89654);
    FqElement r87 = fq_long(0x9cebd04800000000,0x3ac6acff0f245ae7,0xebdf450e6b3ef4a8,0x0e7c4b2a0a9c9ec7);
    FqElement r88 = fq_long(0xc3df73e9278302b9,0x687e956e978e3572,0x39f568c34d3bac22,0x038afc10cb392fc7);
    FqElement r89 = fq_short(0);

    Fq_shr_test(r80, a22, fq_short(0),    80);
    Fq_shr_test(r81, a22, fq_short(1),    81);
    Fq_shr_test(r82, a22, fq_short(12),   82);
    Fq_shr_test(r83, a22, fq_short(32),   83);
    Fq_shr_test(r84, a22, fq_short(132),  84);
    Fq_shr_test(r85, a22, fq_short(432),  85);
    Fq_shr_test(r86, a22, fq_short(-1),   86);
    Fq_shr_test(r87, a22, fq_short(-32),  87);
    Fq_shr_test(r88, a22, fq_short(-132), 88);
    Fq_shr_test(r89, a22, fq_short(-432), 89);
}

void Fq_shl_test(FqElement r_expected, FqElement a, FqElement b, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_shl(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fq_shl_short_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fq_shl_test(fq_short(r_expected), fq_short(a), fq_short(b), index);
}

void Fq_shl_mshort_test(int32_t r_expected, int32_t a, int32_t b, int index)
{
    Fq_shl_test(fq_mshort(r_expected), fq_mshort(a), fq_short(b), index);
}

void Fq_shl_unit_test()
{
    Fq_shl_short_test(        0,     0xa1f0, 0x1bb8,   0);
    Fq_shl_short_test(   0xa1f0,     0xa1f0,       0,  1);
    Fq_shl_short_test(0x000143e0,    0xa1f0,       1,  2);
    Fq_shl_short_test(0x000050f8,    0xa1f0,      -1,  3);
    Fq_shl_short_test(0x000287c0,    0xa1f0,       2,  4);
    Fq_shl_short_test(0x0000287c,    0xa1f0,      -2,  5);
    Fq_shl_short_test(0x0000050f,    0xa1f0,      -5,  6);
    Fq_shl_short_test(0x0a1f0000,    0xa1f0,      12,  7);
    Fq_shl_short_test(      0xa,     0xa1f0,     -12,  8);
    Fq_shl_short_test(        0,     0xa1f0,     -22,  9);
    Fq_shl_short_test(        0,     0xa1f0,     256, 10);


    FqElement a21 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b21 = fq_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a22 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b22 = fq_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a23 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b23 = fq_long(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a24 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b24 = fq_mlong(0xfbb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);

    FqElement a25 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b25 = fq_long(0x1bb8e645ae216da7);

    FqElement a26 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement b26 = fq_mlong(0x1bb8e645ae216da7);


    Fq_shl_test(fq_short(0), a21, b21, 21);
    Fq_shl_test(fq_short(0), a22, b22, 22);
    Fq_shl_test(fq_short(0), a23, b23, 23);
    Fq_shl_test(fq_short(0), a24, b24, 24);
    Fq_shl_test(fq_short(0), a25, b25, 25);
    Fq_shl_test(fq_short(0), a26, b26, 26);


    FqElement r31 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement r32 = fq_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r33 = fq_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FqElement r34 = fq_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FqElement r35 = fq_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FqElement r36 = fq_long(0xbbdf73e9278302b9,0xa55b4db7397f303c,0x8870667812989bc6,0x003481a1faf682b1);
    FqElement r37 = fq_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);
    FqElement r41 = fq_long(0x50f87d64fc000000,0x4a0cfa121e6e5c24,0x6e14116da0605617,0x0c19139cb84c680a);
    FqElement r42 = fq_long(0x450f87d64fc00000,0x74a0cfa121e6e5c2,0xa6e14116da060561,0x00c19139cb84c680);
    FqElement r43 = fq_long(0x848a1f0fac9f8000,0xc2e9419f4243cdcb,0x014dc2822db40c0a,0x000183227397098d);
    FqElement r44 = fq_long(0x72e12287c3eb27e0,0x02b0ba5067d090f3,0x63405370a08b6d03,0x00000060c89ce5c2);
    FqElement r45 = fq_long(0x3cdcb848a1f0fac9,0x40c0ac2e9419f424,0x7098d014dc2822db,0x0000000018322739);
    FqElement r46 = fq_long(0x4dc2822db40c0ac2,0x0183227397098d01,0x0000000000000000,0x0000000000000000);
    FqElement r47 = fq_long(0x0000000000183227,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fq_shl_test(r31,         a21, fq_short(0),    31);
    Fq_shl_test(r32,         a21, fq_short(1),    32);
    Fq_shl_test(r33,         a21, fq_short(5),    33);
    Fq_shl_test(r34,         a21, fq_short(12),   34);
    Fq_shl_test(r35,         a21, fq_short(22),   35);
    Fq_shl_test(r36,         a21, fq_short(32),   36);
    Fq_shl_test(r37,         a21, fq_short(132),  37);
    Fq_shl_test(fq_long(0),  a21, fq_short(232),  38);
    Fq_shl_test(fq_short(0), a21, fq_short(432),  39);

    Fq_shl_test(r41,         a21, fq_short(-1),   41);
    Fq_shl_test(r42,         a21, fq_short(-5),   42);
    Fq_shl_test(r43,         a21, fq_short(-12),  43);
    Fq_shl_test(r44,         a21, fq_short(-22),  44);
    Fq_shl_test(r45,         a21, fq_short(-32),  45);
    Fq_shl_test(r46,         a21, fq_short(-132), 46);
    Fq_shl_test(r47,         a21, fq_short(-232), 47);
    Fq_shl_test(fq_short(0), a21, fq_short(-332), 48);
    Fq_shl_test(fq_short(0), a21, fq_short(-432), 49);

    FqElement r51 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement r52 = fq_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r53 = fq_long(0x3e1f593f00000000,0x833e84879b970914,0x85045b68181585d2,0x0644e72e131a029b);
    FqElement r54 = fq_long(0x0fac9f8000000000,0x9f4243cdcb848a1f,0x822db40c0ac2e941,0x227397098d014dc2);
    FqElement r55 = fq_long(0xb27e000000000000,0x090f372e12287c3e,0xb6d0302b0ba5067d,0x0e5c263405370a08);
    FqElement r56 = fq_long(0xbbdf73e9278302b9,0xa55b4db7397f303c,0x8870667812989bc6,0x003481a1faf682b1);
    FqElement r57 = fq_long(0x0000000000000000,0x0000000000000000,0x1f0fac9f80000000,0x019f4243cdcb848a);

    Fq_shl_test(r51,         a21, fq_long(0),    51);
    Fq_shl_test(r52,         a21, fq_long(1),    52);
    Fq_shl_test(r53,         a21, fq_long(5),    53);
    Fq_shl_test(r54,         a21, fq_long(12),   54);
    Fq_shl_test(r55,         a21, fq_long(22),   55);
    Fq_shl_test(r56,         a21, fq_long(32),   56);
    Fq_shl_test(r57,         a21, fq_long(132),  57);
    Fq_shl_test(fq_long(0),  a21, fq_long(232),  58);
    Fq_shl_test(fq_short(0), a21, fq_long(432),  59);

    Fq_shl_test(fq_short(0), a21, fq_long(-1),   61);
    Fq_shl_test(fq_short(0), a21, fq_long(-5),   62);
    Fq_shl_test(fq_short(0), a21, fq_long(-12),  63);
    Fq_shl_test(fq_short(0), a21, fq_long(-22),  64);
    Fq_shl_test(fq_short(0), a21, fq_long(-32),  65);
    Fq_shl_test(fq_short(0), a21, fq_long(-132), 66);
    Fq_shl_test(fq_short(0), a21, fq_long(-232), 67);
    Fq_shl_test(fq_short(0), a21, fq_long(-332), 68);
    Fq_shl_test(fq_short(0), a21, fq_long(-432), 69);

    Fq_shl_test(fq_short(0), a21, fq_mlong(1),    71);
    Fq_shl_test(fq_short(0), a21, fq_mlong(12),   72);
    Fq_shl_test(fq_short(0), a21, fq_mlong(32),   73);
    Fq_shl_test(fq_short(0), a21, fq_mlong(132),  74);
    Fq_shl_test(fq_short(0), a21, fq_mlong(432),  75);
    Fq_shl_test(fq_short(0), a21, fq_mlong(-1),   76);
    Fq_shl_test(fq_short(0), a21, fq_mlong(-5),   77);
    Fq_shl_test(fq_short(0), a21, fq_mlong(-12),  78);

    FqElement r80 = fq_long(0x0f245ae79cebd048,0x6b3ef4a83ac6acff,0x0a9c9ec7ebdf450e,0x240191410e7c4b2a);
    FqElement r81 = fq_long(0x1e48b5cf39d7a090,0xd67de950758d59fe,0x15393d8fd7be8a1c,0x080322821cf89654);
    FqElement r82 = fq_long(0x45ae79cebd048000,0xef4a83ac6acff0f2,0xc9ec7ebdf450e6b3,0x191410e7c4b2a0a9);
    FqElement r83 = fq_long(0x9cebd04800000000,0x3ac6acff0f245ae7,0xebdf450e6b3ef4a8,0x0e7c4b2a0a9c9ec7);
    FqElement r84 = fq_long(0xc3df73e9278302b9,0x687e956e978e3572,0x39f568c34d3bac22,0x038afc10cb392fc7);
    FqElement r85 = fq_short(0);
    FqElement r86 = fq_long(0x87922d73ce75e824,0x359f7a541d63567f,0x054e4f63f5efa287,0x1200c8a0873e2595);
    FqElement r87 = fq_long(0x3ac6acff0f245ae7,0xebdf450e6b3ef4a8,0x0e7c4b2a0a9c9ec7,0x0000000024019141);
    FqElement r88 = fq_long(0xa0a9c9ec7ebdf450,0x0240191410e7c4b2,0x0000000000000000,0x0000000000000000);
    FqElement r89 = fq_short(0);

    Fq_shl_test(r80, a22, fq_short(0),    80);
    Fq_shl_test(r81, a22, fq_short(1),    81);
    Fq_shl_test(r82, a22, fq_short(12),   82);
    Fq_shl_test(r83, a22, fq_short(32),   83);
    Fq_shl_test(r84, a22, fq_short(132),  84);
    Fq_shl_test(r85, a22, fq_short(432),  85);
    Fq_shl_test(r86, a22, fq_short(-1),   86);
    Fq_shl_test(r87, a22, fq_short(-32),  87);
    Fq_shl_test(r88, a22, fq_short(-132), 88);
    Fq_shl_test(r89, a22, fq_short(-432), 89);
}

void Fq_Rw_Neg_unit_test()
{
    //Fr_Rw_Neg_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0x9a2f914ce07cfd47,0x367766d2b951244,0xdc2822db40c0ac2f,0x183227397098d014};
    //Fr_Rw_Neg_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_Neg_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x3c208c16d87cfd49,0x97816a916871ca8c,0xb85045b68181585d,0x30644e72e131a029};
    //Fr_Rw_Neg_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0x3c208c16d87cfd49,0x97816a916871ca8e,0xb85045b68181585e,0x30644e72e131a02a};
    //Fr_Rw_Neg_test 5:
    FqRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FqRawElement pRawResult5= {0x0,0x0,0x0,0x0};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;
    FqRawElement pRawResult5_c;

    Fq_rawNeg(pRawResult0_c, pRawA0);
    Fq_rawNeg(pRawResult1_c, pRawA1);
    Fq_rawNeg(pRawResult2_c, pRawA2);
    Fq_rawNeg(pRawResult3_c, pRawA3);
    Fq_rawNeg(pRawResult5_c, pRawA5);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_Rw_Neg_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_Rw_Neg_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_Rw_Neg_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_Rw_Neg_unit_test");
    compare_Result(pRawResult5, pRawResult5_c,pRawA5,pRawA5, 5, "Fq_Rw_Neg_unit_test");
}

void Fq_Rw_copy_unit_test()
{
    //Fq_Rw_copy_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    //Fq_Rw_copy_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x1,0x0,0x0,0x0};
    //Fq_Rw_copy_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0xfffffffffffffffe,0x0,0x0,0x0};
    //Fq_Rw_copy_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;

    Fq_rawCopy(pRawResult0_c, pRawA0);
    Fq_rawCopy(pRawResult1_c, pRawA1);
    Fq_rawCopy(pRawResult2_c, pRawA2);
    Fq_rawCopy(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_Rw_copy_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_Rw_copy_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_Rw_copy_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_Rw_copy_unit_test");
}


void Fq_Rw_add_unit_test()
{
    //Fq_rawAdd Test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FqRawElement pRawResult0= {0xbda9e10fa6216da7,0xe8182ed62039122b,0x6871a618947c2cb3,0x1a48f7eaefe714ba};
    //Fq_rawAdd Test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x3,0x0,0x0,0x0};
    //Fq_rawAdd Test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0xfffffffffffffffd,0x1,0x0,0x0};
    //Fq_rawAdd Test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement pRawResult3= {0xc3df73e9278302b6,0x687e956e978e3571,0x47afba497e7ea7a1,0xcf9bb18d1ece5fd5};
    //Fq_rawAdd Test 6:
    FqRawElement pRawA6= {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    FqRawElement pRawB6= {0x0,0x0,0x0,0x0};
    FqRawElement pRawResult6= {0x0,0x0,0x0,0x0};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;
    FqRawElement pRawResult6_c;

    Fq_rawAdd(pRawResult0_c, pRawA0, pRawB0);
    Fq_rawAdd(pRawResult1_c, pRawA1, pRawB1);
    Fq_rawAdd(pRawResult2_c, pRawA2, pRawB2);
    Fq_rawAdd(pRawResult3_c, pRawA3, pRawB3);
    Fq_rawAdd(pRawResult6_c, pRawA6, pRawB6);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawB0,  0, "Fq_Rw_add_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1, pRawB1, 1, "Fq_Rw_add_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2, pRawB2, 2, "Fq_Rw_add_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3, pRawB3, 3, "Fq_Rw_add_unit_test");
    compare_Result(pRawResult6, pRawResult6_c,pRawA6, pRawB6, 6, "Fq_Rw_add_unit_test");
}

void Fq_Rw_sub_unit_test()
{
    //Fq_Rw_sub_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FqRawElement pRawResult0= {0x8638148449de9259,0x401bb97259805e65,0x4fde9f9ded052ba9,0x161b5687f14a8b6f};
    //Fq_Rw_sub_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    //Fq_Rw_sub_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    //Fq_Rw_sub_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement pRawResult3= {0x3c208c16d87cfd46,0x97816a916871ca8c,0xb85045b68181585c,0x30644e72e131a028};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;

    Fq_rawSub(pRawResult0_c, pRawA0, pRawB0);
    Fq_rawSub(pRawResult1_c, pRawA1, pRawB1);
    Fq_rawSub(pRawResult2_c, pRawA2, pRawB2);
    Fq_rawSub(pRawResult3_c, pRawA3, pRawB3);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0, pRawB0, 0, "Fq_Rw_sub_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1, pRawB1, 1, "Fq_Rw_sub_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2, pRawB2, 2, "Fq_Rw_sub_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3, pRawB3, 3, "Fq_Rw_sub_unit_test");
}

void Fq_Rw_mul_unit_test()
{
    //Fq_Rw_mul_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FqRawElement pRawResult0= {0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb};
    //Fq_Rw_mul_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49};
    //Fq_Rw_mul_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x1b332e37e22aea3c,0x6d7519cca22ac926,0xa2b9e2fdbc1f2a77,0x3058d8944ed69677};
    //Fq_Rw_mul_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement pRawResult3= {0x1e51892c7f798de,0x49c1eec88964fb31,0xe7524f2299ec0ee2,0x337a0489fce7555};
    //Fq_Rw_mul_test 4:
    FqRawElement pRawA4= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB4= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult4= {0xebb3da0ac591a7d2,0xdc19acc8059254c6,0xc31f14f32c65f257,0x373ff2663c811ac};
    //Fq_Rw_mul_test 5:
    FqRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FqRawElement pRawB5= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult5= {0x0,0x0,0x0,0x0};
    //Fq_Rw_mul_test 8:
    FqRawElement pRawA8= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB8= {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    FqRawElement pRawResult8= {0x0,0x0,0x0,0x0};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;
    FqRawElement pRawResult4_c;
    FqRawElement pRawResult5_c;
    FqRawElement pRawResult8_c;

    Fq_rawMMul(pRawResult0_c, pRawA0, pRawB0);
    Fq_rawMMul(pRawResult1_c, pRawA1, pRawB1);
    Fq_rawMMul(pRawResult2_c, pRawA2, pRawB2);
    Fq_rawMMul(pRawResult3_c, pRawA3, pRawB3);
    Fq_rawMMul(pRawResult4_c, pRawA4, pRawB4);
    Fq_rawMMul(pRawResult5_c, pRawA5, pRawB5);
    Fq_rawMMul(pRawResult8_c, pRawA8, pRawB8);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0, pRawB0, 0, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1, pRawB1, 1, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2, pRawB2, 2, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3, pRawB3, 3, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult4, pRawResult4_c,pRawA5, pRawB5, 4, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult5, pRawResult5_c,pRawA5, pRawB5, 5, "Fq_Rw_mul_unit_test");
    compare_Result(pRawResult8, pRawResult8_c,pRawA8, pRawB8, 8, "Fq_Rw_mul_unit_test");
}

void Fq_Rw_Msquare_unit_test()
{
    //Fq_Rw_Msquare_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0xac0b6f222f67487d,0x364d764ea56127d9,0xe5ad1f8aa6ef1ae1,0x2dffef30a4034c35};
    //Fq_Rw_Msquare_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0xed84884a014afa37,0xeb2022850278edf8,0xcf63e9cfb74492d9,0x2e67157159e5c639};
    //Fq_Rw_Msquare_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0xcac67c2bcf3f94c9,0xb20d5c033f4b535e,0xad88b23a6703c471,0x3688947d16d07fa};
    //Fq_Rw_Msquare_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0x4c78ebc8ab4ce00d,0xdcbaf4c118eb7001,0x1c8e537a8c87e0f4,0x1fdf7ac5e6e8ec32};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;

    Fq_rawMSquare(pRawResult0_c, pRawA0);
    Fq_rawMSquare(pRawResult1_c, pRawA1);
    Fq_rawMSquare(pRawResult2_c, pRawA2);
    Fq_rawMSquare(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_Rw_Msquare_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_Rw_Msquare_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_Rw_Msquare_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_Rw_Msquare_unit_test");
}

void Fq_Rw_mul1_unit_test()
{
    //Fq_Rw_mul1_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FqRawElement pRawResult0= {0x8b363b7691ff055d,0xb5ada052b1165e8f,0x4b56ee9c6be00e25,0x2cb43dbcbe503199};
    //Fq_Rw_mul1_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49};
    //Fq_Rw_mul1_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x1b332e37e22aea3c,0x6d7519cca22ac926,0xa2b9e2fdbc1f2a77,0x3058d8944ed69677};
    //Fq_Rw_mul1_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement pRawResult3= {0x95b6aeefa3f8e52,0x3bca00aff22ad49,0x78ca497c3a602fb9,0x217bf6416a170b5e};
    //Fq_Rw_mul1_test 9:
    FqRawElement pRawA9= {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    FqRawElement pRawB9= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult9= {0x0,0x0,0x0,0x0};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;
    FqRawElement pRawResult9_c;

    Fq_rawMMul1(pRawResult0_c, pRawA0, pRawB0[0]);
    Fq_rawMMul1(pRawResult1_c, pRawA1, pRawB1[0]);
    Fq_rawMMul1(pRawResult2_c, pRawA2, pRawB2[0]);
    Fq_rawMMul1(pRawResult3_c, pRawA3, pRawB3[0]);
    Fq_rawMMul1(pRawResult9_c, pRawA9, pRawB9[0]);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0, pRawB0, 0, "Fq_Rw_mul1_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1, pRawB1, 1, "Fq_Rw_mul1_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2, pRawB2, 2, "Fq_Rw_mul1_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3, pRawB3, 3, "Fq_Rw_mul1_unit_test");
    compare_Result(pRawResult9, pRawResult9_c,pRawA9, pRawB9, 9, "Fq_Rw_mul1_unit_test");
}

void Fq_Rw_ToMontgomery_unit_test()
{
    //Fq_Rw_ToMontgomery_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0x6a85f78919821592,0x49e80c88cd27dd10,0x386fe049d2e0e036,0xbf6322e9912c187};
    //Fq_Rw_ToMontgomery_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0xd35d438dc58f0d9d,0xa78eb28f5c70b3d,0x666ea36f7879462c,0xe0a77c19a07df2f};
    //Fq_Rw_ToMontgomery_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x86e3b42426baaaf0,0x6f40101ffae5e7b,0x8650e6f06c9181cb,0x546132966296a05};
    //Fq_Rw_ToMontgomery_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0x8898357aa26c8d3a,0xa38cd66a3a80dbbc,0xbe78fcfa9301038b,0x66c76b0259fe60};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;

    Fq_rawToMontgomery(pRawResult0_c, pRawA0);
    Fq_rawToMontgomery(pRawResult1_c, pRawA1);
    Fq_rawToMontgomery(pRawResult2_c, pRawA2);
    Fq_rawToMontgomery(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_Rw_ToMontgomery_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_Rw_ToMontgomery_unit_test");
}

void Fq_Rw_IsEq_unit_test()
{
    //Fq_rawIsEq 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawB0= {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5};
    FqRawElement pRawResult0= {0x0};
    //Fq_rawIsEq 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawB1= {0x2,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x0};
    //Fq_rawIsEq 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawB2= {0xffffffffffffffff,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x0};
    //Fq_rawIsEq 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawB3= {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement pRawResult3= {0x0};
    //Fq_rawIsEq 7:
    FqRawElement pRawA7= {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    FqRawElement pRawB7= {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029};
    FqRawElement pRawResult7= {0x1};

    FqRawElement pRawResult0_c = {0};
    FqRawElement pRawResult1_c = {0};
    FqRawElement pRawResult2_c = {0};
    FqRawElement pRawResult3_c = {0};
    FqRawElement pRawResult7_c = {0};

    pRawResult0_c[0] = Fq_rawIsEq(pRawA0, pRawB0);
    pRawResult1_c[0] = Fq_rawIsEq(pRawA1, pRawB1);
    pRawResult2_c[0] = Fq_rawIsEq(pRawA2, pRawB2);
    pRawResult3_c[0] = Fq_rawIsEq(pRawA3, pRawB3);
    pRawResult7_c[0] = Fq_rawIsEq(pRawA7, pRawB7);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0, pRawB0, 0, "Fq_Rw_IsEq_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1, pRawB1, 1, "Fq_Rw_IsEq_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2, pRawB2, 2, "Fq_Rw_IsEq_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3, pRawB3, 3, "Fq_Rw_IsEq_unit_test");
    compare_Result(pRawResult7, pRawResult7_c,pRawA7, pRawB7, 7, "Fq_Rw_IsEq_unit_test");
}

void Fq_rawIsZero_unit_test()
{
    //Fq_rawIsZero_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0x0};
    //Fq_rawIsZero_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0x0};
    //Fq_rawIsZero_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x0};
    //Fq_rawIsZero_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0x0};
    //Fq_rawIsZero_test 5:
    FqRawElement pRawA5= {0x0,0x0,0x0,0x0};
    FqRawElement pRawResult5= {0x1};

    FqRawElement pRawResult0_c = {0};
    FqRawElement pRawResult1_c = {0};
    FqRawElement pRawResult2_c = {0};
    FqRawElement pRawResult3_c = {0};
    FqRawElement pRawResult5_c = {0};

    pRawResult0_c[0] = Fq_rawIsZero(pRawA0);
    pRawResult1_c[0] = Fq_rawIsZero(pRawA1);
    pRawResult2_c[0] = Fq_rawIsZero(pRawA2);
    pRawResult3_c[0] = Fq_rawIsZero(pRawA3);
    pRawResult5_c[0] = Fq_rawIsZero(pRawA5);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_rawIsZero_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_rawIsZero_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_rawIsZero_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_rawIsZero_unit_test");
    compare_Result(pRawResult5, pRawResult5_c,pRawA5,pRawA5, 5, "Fq_rawIsZero_unit_test");
}

void Fq_Rw_FromMontgomery_unit_test()
{
    //Fq_Rw_FromMontgomery_test 0:
    FqRawElement pRawA0= {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};
    FqRawElement pRawResult0= {0xf245ae79cebd048,0x6b3ef4a83ac6acff,0xa9c9ec7ebdf450e,0x240191410e7c4b2a};
    //Fq_Rw_FromMontgomery_test 1:
    FqRawElement pRawA1= {0x1,0x0,0x0,0x0};
    FqRawElement pRawResult1= {0xed84884a014afa37,0xeb2022850278edf8,0xcf63e9cfb74492d9,0x2e67157159e5c639};
    //Fq_Rw_FromMontgomery_test 2:
    FqRawElement pRawA2= {0xfffffffffffffffe,0x0,0x0,0x0};
    FqRawElement pRawResult2= {0x506cb20c12eb5573,0xbb67bdc962df75c7,0xf53130c3551b6605,0x2cf04f4c7d698e7c};
    //Fq_Rw_FromMontgomery_test 3:
    FqRawElement pRawA3= {0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe,0xfffffffffffffffe};
    FqRawElement pRawResult3= {0x121f6855ad310d9b,0x21ee6f188a0865f2,0x3fbf1ab5ddb67cc1,0x418a171f094820a};

    FqRawElement pRawResult0_c;
    FqRawElement pRawResult1_c;
    FqRawElement pRawResult2_c;
    FqRawElement pRawResult3_c;

    Fq_rawFromMontgomery(pRawResult0_c, pRawA0);
    Fq_rawFromMontgomery(pRawResult1_c, pRawA1);
    Fq_rawFromMontgomery(pRawResult2_c, pRawA2);
    Fq_rawFromMontgomery(pRawResult3_c, pRawA3);

    compare_Result(pRawResult0, pRawResult0_c,pRawA0,pRawA0, 0, "Fq_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult1, pRawResult1_c,pRawA1,pRawA1, 1, "Fq_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult2, pRawResult2_c,pRawA2,pRawA2, 2, "Fq_Rw_FromMontgomery_unit_test");
    compare_Result(pRawResult3, pRawResult3_c,pRawA3,pRawA3, 3, "Fq_Rw_FromMontgomery_unit_test");
}

void Fq_toNormal_unit_test()
{
    //Fq_toNormal_test 0:
    FqElement pA0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult0= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_toNormal_test 1:
    FqElement pA1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult1= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_toNormal_test 2:
    FqElement pA2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    //Fq_toNormal_test 3:
    FqElement pA3= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pResult3= {0x0,0x80000000,{0xf245ae79cebd048,0x6b3ef4a83ac6acff,0xa9c9ec7ebdf450e,0x240191410e7c4b2a}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_toNormal(&Result0_c, &pA0);
    Fq_toNormal(&Result1_c, &pA1);
    Fq_toNormal(&Result2_c, &pA2);
    Fq_toNormal(&Result3_c, &pA3);

    compare_Result(&pResult0, &Result0_c,&pA0,&pA0, 0, "Fq_toNormal_unit_test");
    compare_Result(&pResult1, &Result1_c,&pA1,&pA1, 1, "Fq_toNormal_unit_test");
    compare_Result(&pResult2, &Result2_c,&pA2,&pA2, 2, "Fq_toNormal_unit_test");
    compare_Result(&pResult3, &Result3_c,&pA3,&pA3, 3, "Fq_toNormal_unit_test");
}

void Fq_mul_s1s2_unit_test()
{
    //Fq_mul_s1s2_test 0:
    FqElement pA_s1s20= {0x1,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s20= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s20= {0x0,0x80000000,{0x2,0x0,0x0,0x0}};
    //Fq_mul_s1s2_test 1:
    FqElement pA_s1s21= {0x0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s21= {0x2,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s21= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_s1s2_test 2:
    FqElement pA_s1s22= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s22= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s22= {0x0,0x80000000,{0x1188b480,0x0,0x0,0x0}};
    //Fq_mul_s1s2_test 3:
    FqElement pA_s1s23= {0x7fffffff,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1s23= {0x7fffffff,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1s23= {0x0,0x80000000,{0x3fffffff00000001,0x0,0x0,0x0}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_s1s20, &pB_s1s20);
    Fq_mul(&Result1_c, &pA_s1s21, &pB_s1s21);
    Fq_mul(&Result2_c, &pA_s1s22, &pB_s1s22);
    Fq_mul(&Result3_c, &pA_s1s23, &pB_s1s23);

    compare_Result(&pResult_s1s20, &Result0_c,&pA_s1s20, &pB_s1s20, 0, "Fq_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s21, &Result1_c,&pA_s1s21, &pB_s1s21, 1, "Fq_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s22, &Result2_c,&pA_s1s22, &pB_s1s22, 2, "Fq_mul_s1s2_unit_test");
    compare_Result(&pResult_s1s23, &Result3_c,&pA_s1s23, &pB_s1s23, 3, "Fq_mul_s1s2_unit_test");
}

void Fq_mul_l1nl2n_unit_test()
{
    //Fq_mul_l1nl2n_test 0:
    FqElement pA_l1nl2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n0= {0x0,0xc0000000,{0xa6ba871b8b1e1b3a,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_mul_l1nl2n_test 1:
    FqElement pA_l1nl2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1nl2n_test 2:
    FqElement pA_l1nl2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2n2= {0x0,0xc0000000,{0xcf8964868a91901b,0x7a6a901fa0148d8,0x4db71dbbc02a5dd1,0x16d1da0bfe7853b1}};
    //Fq_mul_l1nl2n_test 3:
    FqElement pA_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2n3= {0x0,0xc0000000,{0xe41f9cbef04da0d3,0x688ae85d2304ac,0x96aa7c6cf3ab1e4f,0x1e0b0a49c35b0816}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1nl2n0, &pB_l1nl2n0);
    Fq_mul(&Result1_c, &pA_l1nl2n1, &pB_l1nl2n1);
    Fq_mul(&Result2_c, &pA_l1nl2n2, &pB_l1nl2n2);
    Fq_mul(&Result3_c, &pA_l1nl2n3, &pB_l1nl2n3);

    compare_Result(&pResult_l1nl2n0, &Result0_c,&pA_l1nl2n0, &pB_l1nl2n0, 0, "Fq_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n1, &Result1_c,&pA_l1nl2n1, &pB_l1nl2n1, 1, "Fq_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n2, &Result2_c,&pA_l1nl2n2, &pB_l1nl2n2, 2, "Fq_mul_l1nl2n_unit_test");
    compare_Result(&pResult_l1nl2n3, &Result3_c,&pA_l1nl2n3, &pB_l1nl2n3, 3, "Fq_mul_l1nl2n_unit_test");
}

void Fq_mul_l1ml2n_unit_test()
{
    //Fq_mul_l1ml2n_test 0:
    FqElement pA_l1ml2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1ml2n_test 1:
    FqElement pA_l1ml2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ml2n_test 2:
    FqElement pA_l1ml2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2n2= {0x0,0x80000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_l1ml2n_test 3:
    FqElement pA_l1ml2n3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2n3= {0x0,0x80000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ml2n0, &pB_l1ml2n0);
    Fq_mul(&Result1_c, &pA_l1ml2n1, &pB_l1ml2n1);
    Fq_mul(&Result2_c, &pA_l1ml2n2, &pB_l1ml2n2);
    Fq_mul(&Result3_c, &pA_l1ml2n3, &pB_l1ml2n3);

    compare_Result(&pResult_l1ml2n0, &Result0_c,&pA_l1ml2n0, &pB_l1ml2n0, 0, "Fq_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n1, &Result1_c,&pA_l1ml2n1, &pB_l1ml2n1, 1, "Fq_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n2, &Result2_c,&pA_l1ml2n2, &pB_l1ml2n2, 2, "Fq_mul_l1ml2n_unit_test");
    compare_Result(&pResult_l1ml2n3, &Result3_c,&pA_l1ml2n3, &pB_l1ml2n3, 3, "Fq_mul_l1ml2n_unit_test");
}

void Fq_mul_l1ml2m_unit_test()
{
    //Fq_mul_l1ml2m_test 0:
    FqElement pA_l1ml2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m0= {0x0,0xc0000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1ml2m_test 1:
    FqElement pA_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ml2m_test 2:
    FqElement pA_l1ml2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ml2m2= {0x0,0xc0000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_l1ml2m_test 3:
    FqElement pA_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ml2m3= {0x0,0xc0000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ml2m0, &pB_l1ml2m0);
    Fq_mul(&Result1_c, &pA_l1ml2m1, &pB_l1ml2m1);
    Fq_mul(&Result2_c, &pA_l1ml2m2, &pB_l1ml2m2);
    Fq_mul(&Result3_c, &pA_l1ml2m3, &pB_l1ml2m3);

    compare_Result(&pResult_l1ml2m0, &Result0_c,&pA_l1ml2m0, &pB_l1ml2m0, 0, "Fq_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m1, &Result1_c,&pA_l1ml2m1, &pB_l1ml2m1, 1, "Fq_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m2, &Result2_c,&pA_l1ml2m2, &pB_l1ml2m2, 2, "Fq_mul_l1ml2m_unit_test");
    compare_Result(&pResult_l1ml2m3, &Result3_c,&pA_l1ml2m3, &pB_l1ml2m3, 3, "Fq_mul_l1ml2m_unit_test");
}

void Fq_mul_l1nl2m_unit_test()
{
    //Fq_mul_l1nl2m_test 0:
    //Fq_mul_l1nl2m_test 0:
    FqElement pA_l1nl2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1nl2m_test 1:
    FqElement pA_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1nl2m_test 2:
    FqElement pA_l1nl2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1nl2m2= {0x0,0x80000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_l1nl2m_test 3:
    FqElement pA_l1nl2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1nl2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1nl2m3= {0x0,0x80000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1nl2m0, &pB_l1nl2m0);
    Fq_mul(&Result1_c, &pA_l1nl2m1, &pB_l1nl2m1);
    Fq_mul(&Result2_c, &pA_l1nl2m2, &pB_l1nl2m2);
    Fq_mul(&Result3_c, &pA_l1nl2m3, &pB_l1nl2m3);

    compare_Result(&pResult_l1nl2m0, &Result0_c,&pA_l1nl2m0, &pB_l1nl2m0, 0, "Fq_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m1, &Result1_c,&pA_l1nl2m1, &pB_l1nl2m1, 1, "Fq_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m2, &Result2_c,&pA_l1nl2m2, &pB_l1nl2m2, 2, "Fq_mul_l1nl2m_unit_test");
    compare_Result(&pResult_l1nl2m3, &Result3_c,&pA_l1nl2m3, &pB_l1nl2m3, 3, "Fq_mul_l1nl2m_unit_test");
}

void Fq_mul_l1ns2n_unit_test()
{
    //Fq_mul_l1ns2n_test 0:
    FqElement pA_l1ns2n0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns2n0= {0x0,0xc0000000,{0xa6ba871b8b1e1b3a,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_mul_l1ns2n_test 1:
    FqElement pA_l1ns2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ns2n_test 2:
    FqElement pA_l1ns2n2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns2n2= {0x0,0xc0000000,{0xba658bb3c5668e7a,0x8b6747b10d51d35a,0x871359d9f90f6f90,0xfd7c8811e0fe4b}};
    //Fq_mul_l1ns2n_test 3:
    FqElement pA_l1ns2n3= {0x7fffffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns2n3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns2n3= {0x0,0xc0000000,{0xe030473272041314,0x5491d21721820941,0x1ec384706e37c635,0x731d84fcf4faa10}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ns2n0, &pB_l1ns2n0);
    Fq_mul(&Result1_c, &pA_l1ns2n1, &pB_l1ns2n1);
    Fq_mul(&Result2_c, &pA_l1ns2n2, &pB_l1ns2n2);
    Fq_mul(&Result3_c, &pA_l1ns2n3, &pB_l1ns2n3);

    compare_Result(&pResult_l1ns2n0, &Result0_c,&pA_l1ns2n0, &pB_l1ns2n0, 0, "Fq_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n1, &Result1_c,&pA_l1ns2n1, &pB_l1ns2n1, 1, "Fq_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n2, &Result2_c,&pA_l1ns2n2, &pB_l1ns2n2, 2, "Fq_mul_l1ns2n_unit_test");
    compare_Result(&pResult_l1ns2n3, &Result3_c,&pA_l1ns2n3, &pB_l1ns2n3, 3, "Fq_mul_l1ns2n_unit_test");
}

void Fq_mul_s1nl2n_unit_test()
{
    //Fq_mul_s1nl2n_test 0:
    FqElement pA_s1nl2n0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1nl2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2n0= {0x0,0xc0000000,{0xa6ba871b8b1e1b3a,0x14f1d651eb8e167b,0xccdd46def0f28c58,0x1c14ef83340fbe5e}};
    //Fq_mul_s1nl2n_test 1:
    FqElement pA_s1nl2n1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1nl2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_s1nl2n_test 2:
    FqElement pA_s1nl2n2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1nl2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1nl2n2= {0x0,0xc0000000,{0xa1ebd3b0c50a79a5,0x991c1c5109e913a5,0x556dc7319816b73,0x12e84d0df59a5777}};
    //Fq_mul_s1nl2n_test 3:
    FqElement pA_s1nl2n3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1nl2n3= {0x7fffffff,0x80000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_s1nl2n3= {0x0,0xc0000000,{0xf7d471598746b6aa,0xc5baff5c4b315cae,0x5913c7393800d697,0x3030eabd6004a0f9}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_s1nl2n0, &pB_s1nl2n0);
    Fq_mul(&Result1_c, &pA_s1nl2n1, &pB_s1nl2n1);
    Fq_mul(&Result2_c, &pA_s1nl2n2, &pB_s1nl2n2);
    Fq_mul(&Result3_c, &pA_s1nl2n3, &pB_s1nl2n3);

    compare_Result(&pResult_s1nl2n0, &Result0_c,&pA_s1nl2n0, &pB_s1nl2n0, 0, "Fq_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n1, &Result1_c,&pA_s1nl2n1, &pB_s1nl2n1, 1, "Fq_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n2, &Result2_c,&pA_s1nl2n2, &pB_s1nl2n2, 2, "Fq_mul_s1nl2n_unit_test");
    compare_Result(&pResult_s1nl2n3, &Result3_c,&pA_s1nl2n3, &pB_s1nl2n3, 3, "Fq_mul_s1nl2n_unit_test");
}

void Fq_mul_s1nl2m_unit_test()
{
    //Fq_mul_s1nl2m_test 0:
    FqElement pA_s1nl2m0= {0x1,0x0,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1nl2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_s1nl2m_test 1:
    FqElement pA_s1nl2m1= {0x0,0x0,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1nl2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1nl2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_s1nl2m_test 2:
    FqElement pA_s1nl2m2= {0xa1f0,0x0,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1nl2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1nl2m2= {0x0,0x80000000,{0xf8fb48ccc33018d3,0xc94964a5af8c4718,0x1a3ee6c0af9b914e,0x137994681281dfa3}};
    //Fq_mul_s1nl2m_test 3:
    FqElement pA_s1nl2m3= {-1,0x0,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1nl2m3= {0x7fffffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_s1nl2m3= {0x0,0x80000000,{0x950091e095a5f7d6,0x3ac97dbc6f34b24d,0xbc48958051e56dce,0x1625d680784e8f0f}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_s1nl2m0, &pB_s1nl2m0);
    Fq_mul(&Result1_c, &pA_s1nl2m1, &pB_s1nl2m1);
    Fq_mul(&Result2_c, &pA_s1nl2m2, &pB_s1nl2m2);
    Fq_mul(&Result3_c, &pA_s1nl2m3, &pB_s1nl2m3);

    compare_Result(&pResult_s1nl2m0, &Result0_c,&pA_s1nl2m0, &pB_s1nl2m0, 0, "Fq_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m1, &Result1_c,&pA_s1nl2m1, &pB_s1nl2m1, 1, "Fq_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m2, &Result2_c,&pA_s1nl2m2, &pB_s1nl2m2, 2, "Fq_mul_s1nl2m_unit_test");
    compare_Result(&pResult_s1nl2m3, &Result3_c,&pA_s1nl2m3, &pB_s1nl2m3, 3, "Fq_mul_s1nl2m_unit_test");
}

void Fq_mul_l1ms2n_unit_test()
{
    //Fq_mul_l1ms2n_test 0:
    FqElement pA_l1ms2n0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2n0= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1ms2n_test 1:
    FqElement pA_l1ms2n1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2n1= {0x2,0x0,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ms2n_test 2:
    FqElement pA_l1ms2n2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2n2= {0x1bb8,0x0,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2n2= {0x0,0x80000000,{0xd48ef8eb6f0a70a7,0x83590aa4708b6780,0x6603a7198a84f5b5,0x27049057c6edb906}};
    //Fq_mul_l1ms2n_test 3:
    FqElement pA_l1ms2n3= {0xffff,0xc0000000,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pB_l1ms2n3= {-1,0x0,{0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff,0x7fffffffffffffff}};
    FqElement pResult_l1ms2n3= {0x0,0x80000000,{0x950091e095a5f7d6,0x3ac97dbc6f34b24d,0xbc48958051e56dce,0x1625d680784e8f0f}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ms2n0, &pB_l1ms2n0);
    Fq_mul(&Result1_c, &pA_l1ms2n1, &pB_l1ms2n1);
    Fq_mul(&Result2_c, &pA_l1ms2n2, &pB_l1ms2n2);
    Fq_mul(&Result3_c, &pA_l1ms2n3, &pB_l1ms2n3);

    compare_Result(&pResult_l1ms2n0, &Result0_c,&pA_l1ms2n0, &pB_l1ms2n0, 0, "Fq_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n1, &Result1_c,&pA_l1ms2n1, &pB_l1ms2n1, 1, "Fq_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n2, &Result2_c,&pA_l1ms2n2, &pB_l1ms2n2, 2, "Fq_mul_l1ms2n_unit_test");
    compare_Result(&pResult_l1ms2n3, &Result3_c,&pA_l1ms2n3, &pB_l1ms2n3, 3, "Fq_mul_l1ms2n_unit_test");
}

void Fq_mul_l1ns2m_unit_test()
{
    //Fq_mul_l1ns2m_test 0:
    FqElement pA_l1ns2m0= {0x1,0x80000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ns2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns2m0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1ns2m_test 1:
    FqElement pA_l1ns2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ns2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ns2m1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ns2m_test 2:
    FqElement pA_l1ns2m2= {0xa1f0,0x80000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ns2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ns2m2= {0x0,0x80000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_l1ns2m_test 3:
    FqElement pA_l1ns2m3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ns2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ns2m3= {0x0,0x80000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ns2m0, &pB_l1ns2m0);
    Fq_mul(&Result1_c, &pA_l1ns2m1, &pB_l1ns2m1);
    Fq_mul(&Result2_c, &pA_l1ns2m2, &pB_l1ns2m2);
    Fq_mul(&Result3_c, &pA_l1ns2m3, &pB_l1ns2m3);

    compare_Result(&pResult_l1ns2m0, &Result0_c,&pA_l1ns2m0, &pB_l1ns2m0, 0, "Fq_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m1, &Result1_c,&pA_l1ns2m1, &pB_l1ns2m1, 1, "Fq_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m2, &Result2_c,&pA_l1ns2m2, &pB_l1ns2m2, 2, "Fq_mul_l1ns2m_unit_test");
    compare_Result(&pResult_l1ns2m3, &Result3_c,&pA_l1ns2m3, &pB_l1ns2m3, 3, "Fq_mul_l1ns2m_unit_test");
}

void Fq_mul_l1ms2m_unit_test()
{
    //Fq_mul_l1ms2m_test 0:
    FqElement pA_l1ms2m0= {0x1,0xc0000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_l1ms2m0= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m0= {0x0,0xc0000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_l1ms2m_test 1:
    FqElement pA_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_l1ms2m1= {0x2,0x40000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_l1ms2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_l1ms2m_test 2:
    FqElement pA_l1ms2m2= {0xa1f0,0xc0000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_l1ms2m2= {0x1bb8,0x40000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_l1ms2m2= {0x0,0xc0000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_l1ms2m_test 3:
    FqElement pA_l1ms2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_l1ms2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_l1ms2m3= {0x0,0xc0000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_l1ms2m0, &pB_l1ms2m0);
    Fq_mul(&Result1_c, &pA_l1ms2m1, &pB_l1ms2m1);
    Fq_mul(&Result2_c, &pA_l1ms2m2, &pB_l1ms2m2);
    Fq_mul(&Result3_c, &pA_l1ms2m3, &pB_l1ms2m3);

    compare_Result(&pResult_l1ms2m0, &Result0_c,&pA_l1ms2m0, &pB_l1ms2m0, 0, "Fq_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m1, &Result1_c,&pA_l1ms2m1, &pB_l1ms2m1, 1, "Fq_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m2, &Result2_c,&pA_l1ms2m2, &pB_l1ms2m2, 2, "Fq_mul_l1ms2m_unit_test");
    compare_Result(&pResult_l1ms2m3, &Result3_c,&pA_l1ms2m3, &pB_l1ms2m3, 3, "Fq_mul_l1ms2m_unit_test");
}

void Fq_mul_s1ml2m_unit_test()
{
    //Fq_mul_s1ml2m_test 0:
    FqElement pA_s1ml2m0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1ml2m0= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m0= {0x0,0xc0000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_s1ml2m_test 1:
    FqElement pA_s1ml2m1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1ml2m1= {0x2,0xc0000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2m1= {0x0,0xc0000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_s1ml2m_test 2:
    FqElement pA_s1ml2m2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1ml2m2= {0x1bb8,0xc0000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1ml2m2= {0x0,0xc0000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_s1ml2m_test 3:
    FqElement pA_s1ml2m3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1ml2m3= {0xffff,0xc0000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1ml2m3= {0x0,0xc0000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_s1ml2m0, &pB_s1ml2m0);
    Fq_mul(&Result1_c, &pA_s1ml2m1, &pB_s1ml2m1);
    Fq_mul(&Result2_c, &pA_s1ml2m2, &pB_s1ml2m2);
    Fq_mul(&Result3_c, &pA_s1ml2m3, &pB_s1ml2m3);

    compare_Result(&pResult_s1ml2m0, &Result0_c,&pA_s1ml2m0, &pB_s1ml2m0, 0, "Fq_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m1, &Result1_c,&pA_s1ml2m1, &pB_s1ml2m1, 1, "Fq_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m2, &Result2_c,&pA_s1ml2m2, &pB_s1ml2m2, 2, "Fq_mul_s1ml2m_unit_test");
    compare_Result(&pResult_s1ml2m3, &Result3_c,&pA_s1ml2m3, &pB_s1ml2m3, 3, "Fq_mul_s1ml2m_unit_test");
}

void Fq_mul_s1ml2n_unit_test()
{
    //Fq_mul_s1ml2n_test 0:
    FqElement pA_s1ml2n0= {0x1,0x40000000,{0x1,0x0,0x0,0x0}};
    FqElement pB_s1ml2n0= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2n0= {0x0,0x80000000,{0x9ee8847d2a18f727,0x3ebeda789c801164,0xe6778de8ed07cd56,0x2c69dc6fd299ec49}};
    //Fq_mul_s1ml2n_test 1:
    FqElement pA_s1ml2n1= {0x0,0x40000000,{0x0,0x0,0x0,0x0}};
    FqElement pB_s1ml2n1= {0x2,0x80000000,{0x2,0x0,0x0,0x0}};
    FqElement pResult_s1ml2n1= {0x0,0x80000000,{0x0,0x0,0x0,0x0}};
    //Fq_mul_s1ml2n_test 2:
    FqElement pA_s1ml2n2= {0xa1f0,0x40000000,{0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014}};
    FqElement pB_s1ml2n2= {0x1bb8,0x80000000,{0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5}};
    FqElement pResult_s1ml2n2= {0x0,0x80000000,{0x1187da3e296269a8,0xd0139eb206e57eeb,0xdb5973382f0e9301,0x2e40d99a3c8089fb}};
    //Fq_mul_s1ml2n_test 3:
    FqElement pA_s1ml2n3= {0xffff,0x40000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pB_s1ml2n3= {0xffff,0x80000000,{0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff}};
    FqElement pResult_s1ml2n3= {0x0,0x80000000,{0xc5687d1b9df6a369,0xbf4f15d4ad74848f,0x3e250df1ad65c620,0x55308b909171d71}};

    FqElement Result0_c = {0,0,{0,0,0,0}};
    FqElement Result1_c = {0,0,{0,0,0,0}};
    FqElement Result2_c= {0,0,{0,0,0,0}};
    FqElement Result3_c= {0,0,{0,0,0,0}};

    Fq_mul(&Result0_c, &pA_s1ml2n0, &pB_s1ml2n0);
    Fq_mul(&Result1_c, &pA_s1ml2n1, &pB_s1ml2n1);
    Fq_mul(&Result2_c, &pA_s1ml2n2, &pB_s1ml2n2);
    Fq_mul(&Result3_c, &pA_s1ml2n3, &pB_s1ml2n3);

    compare_Result(&pResult_s1ml2n0, &Result0_c,&pA_s1ml2n0, &pB_s1ml2n0, 0, "Fq_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n1, &Result1_c,&pA_s1ml2n1, &pB_s1ml2n1, 1, "Fq_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n2, &Result2_c,&pA_s1ml2n2, &pB_s1ml2n2, 2, "Fq_mul_s1ml2n_unit_test");
    compare_Result(&pResult_s1ml2n3, &Result3_c,&pA_s1ml2n3, &pB_s1ml2n3, 3, "Fq_mul_s1ml2n_unit_test");
}

void Fq_rawCopyS2L_test(FqRawElement r_expected, int64_t a, int idx)
{
#if !(defined(USE_ASM) && defined(ARCH_X86_64))

    FqRawElement r_computed = {0xb,0xa,0xd,0xd};

    Fq_rawCopyS2L(r_computed, a);

    compare_Result(r_expected, r_computed, a, idx, __func__);
#endif
}

void Fq_rawCopyS2L_unit_test()
{
    int64_t      a0 = 1;
    FqRawElement r0 = {1,0,0,0};

    int64_t      a1 = -1;
    FqRawElement r1 = {0x3c208c16d87cfd46, 0x97816a916871ca8d, 0xb85045b68181585d, 0x30644e72e131a029};

    int64_t      a2 = -2224;
    FqRawElement r2 = {0x3c208c16d87cf497, 0x97816a916871ca8d, 0xb85045b68181585d, 0x30644e72e131a029};

    int64_t      a3 = 0;
    FqRawElement r3 = {0,0,0,0};

    int64_t      a4 =  2224;
    FqRawElement r4 = {2224,0,0,0};

    Fq_rawCopyS2L_test(r0, a0, 0);
    Fq_rawCopyS2L_test(r1, a1, 1);
    Fq_rawCopyS2L_test(r2, a2, 2);
    Fq_rawCopyS2L_test(r3, a3, 3);
    Fq_rawCopyS2L_test(r4, a4, 4);
}

void Fr_rawCopyS2L_test(FrRawElement r_expected, int64_t a, int idx)
{
#if !(defined(USE_ASM) && defined(ARCH_X86_64))

    FrRawElement r_computed = {0xb,0xa,0xd,0xd};

    Fr_rawCopyS2L(r_computed, a);

    compare_Result(r_expected, r_computed, a, idx, __func__);
#endif
}

void Fr_rawCopyS2L_unit_test()
{
    int64_t      a0 = 1;
    FrRawElement r0 = {1,0,0,0};

    int64_t      a1 = -1;
    FrRawElement r1 = {0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};

    int64_t      a2 = -2224;
    FrRawElement r2 = {0x43e1f593effff751,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029};

    int64_t      a3 = 0;
    FrRawElement r3 = {0,0,0,0};

    int64_t      a4 =  2224;
    FrRawElement r4 = {2224,0,0,0};

    Fr_rawCopyS2L_test(r0, a0, 0);
    Fr_rawCopyS2L_test(r1, a1, 1);
    Fr_rawCopyS2L_test(r2, a2, 2);
    Fr_rawCopyS2L_test(r3, a3, 3);
    Fr_rawCopyS2L_test(r4, a4, 4);
}

void Fr_rawShr_test(FrRawElement r_expected, FrRawElement a, uint64_t b)
{
    FrRawElement r_computed = {0xbadbadbadbadbadb,0xadbadbadbadbadba,0xdbadbadbadbadbad,0xbadbadbadbadbadb};

    Fr_rawShr(r_computed, a, b);

    compare_Result(r_expected, r_computed, a, b, (int)b, __func__);
}

void Fr_rawShr_unit_test()
{
    FrRawElement rawA1     = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement rawA2     = {0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa};

    FrRawElement result1   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x7fffffffffffffff};
    FrRawElement result2   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x3fffffffffffffff};
    FrRawElement result3   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x1fffffffffffffff};
    FrRawElement result4   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0fffffffffffffff};

    FrRawElement result7   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x01ffffffffffffff};
    FrRawElement result8   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00ffffffffffffff};
    FrRawElement result9   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x007fffffffffffff};

    FrRawElement result15  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0001ffffffffffff};
    FrRawElement result16  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000ffffffffffff};
    FrRawElement result17  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00007fffffffffff};

    FrRawElement result30  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000003ffffffff};
    FrRawElement result31  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000001ffffffff};
    FrRawElement result32  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000000ffffffff};
    FrRawElement result33  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x000000007fffffff};
    FrRawElement result34  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x000000003fffffff};

    FrRawElement result63  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000000000000001};
    FrRawElement result64  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000000000000000};
    FrRawElement result65  = {0xffffffffffffffff,0xffffffffffffffff,0x7fffffffffffffff,0x0000000000000000};

    FrRawElement result95  = {0xffffffffffffffff,0xffffffffffffffff,0x00000001ffffffff,0x0000000000000000};
    FrRawElement result96  = {0xffffffffffffffff,0xffffffffffffffff,0x00000000ffffffff,0x0000000000000000};
    FrRawElement result97  = {0xffffffffffffffff,0xffffffffffffffff,0x000000007fffffff,0x0000000000000000};

    FrRawElement result127 = {0xffffffffffffffff,0xffffffffffffffff,0x0000000000000001,0x0000000000000000};
    FrRawElement result128 = {0xffffffffffffffff,0xffffffffffffffff,0x0000000000000000,0x0000000000000000};
    FrRawElement result129 = {0xffffffffffffffff,0x7fffffffffffffff,0x0000000000000000,0x0000000000000000};

    FrRawElement result159 = {0x5555555555555555,0x0000000155555555,0x0000000000000000,0x0000000000000000};
    FrRawElement result160 = {0xaaaaaaaaaaaaaaaa,0x00000000aaaaaaaa,0x0000000000000000,0x0000000000000000};
    FrRawElement result161 = {0x5555555555555555,0x0000000055555555,0x0000000000000000,0x0000000000000000};

    FrRawElement result191 = {0x5555555555555555,0x0000000000000001,0x0000000000000000,0x0000000000000000};
    FrRawElement result192 = {0xaaaaaaaaaaaaaaaa,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result193 = {0x5555555555555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    FrRawElement result223 = {0x0000000155555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result224 = {0x00000000aaaaaaaa,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result225 = {0x0000000055555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    FrRawElement result250 = {0x000000000000003f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result251 = {0x000000000000001f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result252 = {0x000000000000000f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FrRawElement result253 = {0x0000000000000007,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    Fr_rawShr_test(result1, rawA1, 1);
    Fr_rawShr_test(result2, rawA1, 2);
    Fr_rawShr_test(result3, rawA1, 3);
    Fr_rawShr_test(result4, rawA1, 4);

    Fr_rawShr_test(result7, rawA1, 7);
    Fr_rawShr_test(result8, rawA1, 8);
    Fr_rawShr_test(result9, rawA1, 9);

    Fr_rawShr_test(result15, rawA1, 15);
    Fr_rawShr_test(result16, rawA1, 16);
    Fr_rawShr_test(result17, rawA1, 17);

    Fr_rawShr_test(result30, rawA1, 30);
    Fr_rawShr_test(result31, rawA1, 31);
    Fr_rawShr_test(result32, rawA1, 32);
    Fr_rawShr_test(result33, rawA1, 33);
    Fr_rawShr_test(result34, rawA1, 34);

    Fr_rawShr_test(result63, rawA1, 63);
    Fr_rawShr_test(result64, rawA1, 64);
    Fr_rawShr_test(result65, rawA1, 65);

    Fr_rawShr_test(result95, rawA1, 95);
    Fr_rawShr_test(result96, rawA1, 96);
    Fr_rawShr_test(result97, rawA1, 97);

    Fr_rawShr_test(result127, rawA1, 127);
    Fr_rawShr_test(result128, rawA1, 128);
    Fr_rawShr_test(result129, rawA1, 129);

    Fr_rawShr_test(result159, rawA2, 159);
    Fr_rawShr_test(result160, rawA2, 160);
    Fr_rawShr_test(result161, rawA2, 161);

    Fr_rawShr_test(result191, rawA2, 191);
    Fr_rawShr_test(result192, rawA2, 192);
    Fr_rawShr_test(result193, rawA2, 193);

    Fr_rawShr_test(result223, rawA2, 223);
    Fr_rawShr_test(result224, rawA2, 224);
    Fr_rawShr_test(result225, rawA2, 225);

    Fr_rawShr_test(result250, rawA1, 250);
    Fr_rawShr_test(result251, rawA1, 251);
    Fr_rawShr_test(result252, rawA1, 252);
    Fr_rawShr_test(result253, rawA1, 253);
}

void Fr_rawShl_test(FrRawElement r_expected, FrRawElement a, uint64_t b)
{
    FrRawElement r_computed = {0xbadbadbadbadbadb,0xadbadbadbadbadba,0xdbadbadbadbadbad,0xbadbadbadbadbadb};

    Fr_rawShl(r_computed, a, b);

    compare_Result(r_expected, r_computed, a, b, (int)b, __func__);
}

void Fr_rawShl_unit_test()
{
    FrRawElement rawA1     = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FrRawElement rawA2     = {0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa};

    FrRawElement result1   = {0xbc1e0a6c0ffffffd,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result2   = {0xbc1e0a6c0ffffffb,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result3   = {0xbc1e0a6c0ffffff7,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result4   = {0xbc1e0a6c0fffffef,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result7   = {0xbc1e0a6c0fffff7f,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result8   = {0xbc1e0a6c0ffffeff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result9   = {0xbc1e0a6c0ffffdff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result15  = {0xbc1e0a6c0fff7fff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result16  = {0xbc1e0a6c0ffeffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result17  = {0xbc1e0a6c0ffdffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result30  = {0xbc1e0a6bcfffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result31  = {0xbc1e0a6b8fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result32  = {0xbc1e0a6b0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result33  = {0xbc1e0a6a0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result34  = {0xbc1e0a680fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result63  = {0x3c1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result64  = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6d,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result65  = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6c,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result95  = {0xbc1e0a6c0fffffff,0xd7cc17b706468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result96  = {0xbc1e0a6c0fffffff,0xd7cc17b686468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result97  = {0xbc1e0a6c0fffffff,0xd7cc17b586468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FrRawElement result127 = {0xbc1e0a6c0fffffff,0x57cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FrRawElement result128 = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a1,0x0f9bb18d1ece5fd6};
    FrRawElement result129 = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a0,0x0f9bb18d1ece5fd6};

    FrRawElement result159 = {0x0000000000000000,0x0000000000000000,0x5555555500000000,0x1555555555555555};
    FrRawElement result160 = {0x0000000000000000,0x0000000000000000,0xaaaaaaaa00000000,0x2aaaaaaaaaaaaaaa};
    FrRawElement result161 = {0x0000000000000000,0x0000000000000000,0x5555555400000000,0x1555555555555555};

    FrRawElement result191 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555555555555};
    FrRawElement result192 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2aaaaaaaaaaaaaaa};
    FrRawElement result193 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555555555554};

    FrRawElement result223 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555500000000};
    FrRawElement result224 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2aaaaaaa00000000};
    FrRawElement result225 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555400000000};

    FrRawElement result250 = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0b9bb18d1ece5fd6};
    FrRawElement result251 = {0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x079bb18d1ece5fd6};
    FrRawElement result252 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x3000000000000000};
    FrRawElement result253 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2000000000000000};

    Fr_rawShl_test(result1, rawA1, 1);
    Fr_rawShl_test(result2, rawA1, 2);
    Fr_rawShl_test(result3, rawA1, 3);
    Fr_rawShl_test(result4, rawA1, 4);

    Fr_rawShl_test(result7, rawA1, 7);
    Fr_rawShl_test(result8, rawA1, 8);
    Fr_rawShl_test(result9, rawA1, 9);

    Fr_rawShl_test(result15, rawA1, 15);
    Fr_rawShl_test(result16, rawA1, 16);
    Fr_rawShl_test(result17, rawA1, 17);

    Fr_rawShl_test(result30, rawA1, 30);
    Fr_rawShl_test(result31, rawA1, 31);
    Fr_rawShl_test(result32, rawA1, 32);
    Fr_rawShl_test(result33, rawA1, 33);
    Fr_rawShl_test(result34, rawA1, 34);

    Fr_rawShl_test(result63, rawA1, 63);
    Fr_rawShl_test(result64, rawA1, 64);
    Fr_rawShl_test(result65, rawA1, 65);

    Fr_rawShl_test(result95, rawA1, 95);
    Fr_rawShl_test(result96, rawA1, 96);
    Fr_rawShl_test(result97, rawA1, 97);

    Fr_rawShl_test(result127, rawA1, 127);
    Fr_rawShl_test(result128, rawA1, 128);
    Fr_rawShl_test(result129, rawA1, 129);

    Fr_rawShl_test(result159, rawA2, 159);
    Fr_rawShl_test(result160, rawA2, 160);
    Fr_rawShl_test(result161, rawA2, 161);

    Fr_rawShl_test(result191, rawA2, 191);
    Fr_rawShl_test(result192, rawA2, 192);
    Fr_rawShl_test(result193, rawA2, 193);

    Fr_rawShl_test(result223, rawA2, 223);
    Fr_rawShl_test(result224, rawA2, 224);
    Fr_rawShl_test(result225, rawA2, 225);

    Fr_rawShl_test(result250, rawA1, 250);
    Fr_rawShl_test(result251, rawA1, 251);
    Fr_rawShl_test(result252, rawA1, 252);
    Fr_rawShl_test(result253, rawA1, 253);
}


void Fq_rawShr_test(FqRawElement r_expected, FqRawElement a, uint64_t b)
{
    FqRawElement r_computed = {0xbadbadbadbadbadb,0xadbadbadbadbadba,0xdbadbadbadbadbad,0xbadbadbadbadbadb};

    Fq_rawShr(r_computed, a, b);

    compare_Result(r_expected, r_computed, a, b, (int)b, __func__);
}

void Fq_rawShr_unit_test()
{
    FqRawElement rawA1     = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement rawA2     = {0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa};

    FqRawElement result1   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x7fffffffffffffff};
    FqRawElement result2   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x3fffffffffffffff};
    FqRawElement result3   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x1fffffffffffffff};
    FqRawElement result4   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0fffffffffffffff};

    FqRawElement result7   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x01ffffffffffffff};
    FqRawElement result8   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00ffffffffffffff};
    FqRawElement result9   = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x007fffffffffffff};

    FqRawElement result15  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0001ffffffffffff};
    FqRawElement result16  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000ffffffffffff};
    FqRawElement result17  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00007fffffffffff};

    FqRawElement result30  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000003ffffffff};
    FqRawElement result31  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000001ffffffff};
    FqRawElement result32  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x00000000ffffffff};
    FqRawElement result33  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x000000007fffffff};
    FqRawElement result34  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x000000003fffffff};

    FqRawElement result63  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000000000000001};
    FqRawElement result64  = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0x0000000000000000};
    FqRawElement result65  = {0xffffffffffffffff,0xffffffffffffffff,0x7fffffffffffffff,0x0000000000000000};

    FqRawElement result95  = {0xffffffffffffffff,0xffffffffffffffff,0x00000001ffffffff,0x0000000000000000};
    FqRawElement result96  = {0xffffffffffffffff,0xffffffffffffffff,0x00000000ffffffff,0x0000000000000000};
    FqRawElement result97  = {0xffffffffffffffff,0xffffffffffffffff,0x000000007fffffff,0x0000000000000000};

    FqRawElement result127 = {0xffffffffffffffff,0xffffffffffffffff,0x0000000000000001,0x0000000000000000};
    FqRawElement result128 = {0xffffffffffffffff,0xffffffffffffffff,0x0000000000000000,0x0000000000000000};
    FqRawElement result129 = {0xffffffffffffffff,0x7fffffffffffffff,0x0000000000000000,0x0000000000000000};

    FqRawElement result159 = {0x5555555555555555,0x0000000155555555,0x0000000000000000,0x0000000000000000};
    FqRawElement result160 = {0xaaaaaaaaaaaaaaaa,0x00000000aaaaaaaa,0x0000000000000000,0x0000000000000000};
    FqRawElement result161 = {0x5555555555555555,0x0000000055555555,0x0000000000000000,0x0000000000000000};

    FqRawElement result191 = {0x5555555555555555,0x0000000000000001,0x0000000000000000,0x0000000000000000};
    FqRawElement result192 = {0xaaaaaaaaaaaaaaaa,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result193 = {0x5555555555555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    FqRawElement result223 = {0x0000000155555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result224 = {0x00000000aaaaaaaa,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result225 = {0x0000000055555555,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    FqRawElement result250 = {0x000000000000003f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result251 = {0x000000000000001f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result252 = {0x000000000000000f,0x0000000000000000,0x0000000000000000,0x0000000000000000};
    FqRawElement result253 = {0x0000000000000007,0x0000000000000000,0x0000000000000000,0x0000000000000000};

    Fq_rawShr_test(result1, rawA1, 1);
    Fq_rawShr_test(result2, rawA1, 2);
    Fq_rawShr_test(result3, rawA1, 3);
    Fq_rawShr_test(result4, rawA1, 4);

    Fq_rawShr_test(result7, rawA1, 7);
    Fq_rawShr_test(result8, rawA1, 8);
    Fq_rawShr_test(result9, rawA1, 9);

    Fq_rawShr_test(result15, rawA1, 15);
    Fq_rawShr_test(result16, rawA1, 16);
    Fq_rawShr_test(result17, rawA1, 17);

    Fq_rawShr_test(result30, rawA1, 30);
    Fq_rawShr_test(result31, rawA1, 31);
    Fq_rawShr_test(result32, rawA1, 32);
    Fq_rawShr_test(result33, rawA1, 33);
    Fq_rawShr_test(result34, rawA1, 34);

    Fq_rawShr_test(result63, rawA1, 63);
    Fq_rawShr_test(result64, rawA1, 64);
    Fq_rawShr_test(result65, rawA1, 65);

    Fq_rawShr_test(result95, rawA1, 95);
    Fq_rawShr_test(result96, rawA1, 96);
    Fq_rawShr_test(result97, rawA1, 97);

    Fq_rawShr_test(result127, rawA1, 127);
    Fq_rawShr_test(result128, rawA1, 128);
    Fq_rawShr_test(result129, rawA1, 129);

    Fq_rawShr_test(result159, rawA2, 159);
    Fq_rawShr_test(result160, rawA2, 160);
    Fq_rawShr_test(result161, rawA2, 161);

    Fq_rawShr_test(result191, rawA2, 191);
    Fq_rawShr_test(result192, rawA2, 192);
    Fq_rawShr_test(result193, rawA2, 193);

    Fq_rawShr_test(result223, rawA2, 223);
    Fq_rawShr_test(result224, rawA2, 224);
    Fq_rawShr_test(result225, rawA2, 225);

    Fq_rawShr_test(result250, rawA1, 250);
    Fq_rawShr_test(result251, rawA1, 251);
    Fq_rawShr_test(result252, rawA1, 252);
    Fq_rawShr_test(result253, rawA1, 253);
}

void Fq_rawShl_test(FqRawElement r_expected, FqRawElement a, uint64_t b)
{
    FqRawElement r_computed = {0xbadbadbadbadbadb,0xadbadbadbadbadba,0xdbadbadbadbadbad,0xbadbadbadbadbadb};

    Fq_rawShl(r_computed, a, b);

    compare_Result(r_expected, r_computed, a, b, (int)b, __func__);
}

void Fq_rawShl_unit_test()
{
    FqRawElement rawA1     = {0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff};
    FqRawElement rawA2     = {0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa};

    FqRawElement result1   = {0xc3df73e9278302b7,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result2   = {0xc3df73e9278302b5,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result3   = {0xc3df73e9278302b1,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result4   = {0xc3df73e9278302a9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result7   = {0xc3df73e927830239,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result8   = {0xc3df73e9278301b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result9   = {0xc3df73e9278300b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result15  = {0xc3df73e9278282b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result16  = {0xc3df73e9278202b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result17  = {0xc3df73e9278102b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result30  = {0xc3df73e8e78302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result31  = {0xc3df73e8a78302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result32  = {0xc3df73e8278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result33  = {0xc3df73e7278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result34  = {0xc3df73e5278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result63  = {0x43df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result64  = {0xc3df73e9278302b9,0x687e956e978e3571,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result65  = {0xc3df73e9278302b9,0x687e956e978e3570,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result95  = {0xc3df73e9278302b9,0x687e956e178e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result96  = {0xc3df73e9278302b9,0x687e956d978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};
    FqRawElement result97  = {0xc3df73e9278302b9,0x687e956c978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6};

    FqRawElement result127 = {0xc3df73e9278302b9,0xe87e956e978e3572,0x47afba497e7ea7a1,0x0f9bb18d1ece5fd6};
    FqRawElement result128 = {0xc3df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a1,0x0f9bb18d1ece5fd6};
    FqRawElement result129 = {0xc3df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a0,0x0f9bb18d1ece5fd6};

    FqRawElement result159 = {0x0000000000000000,0x0000000000000000,0x5555555500000000,0x1555555555555555};
    FqRawElement result160 = {0x0000000000000000,0x0000000000000000,0xaaaaaaaa00000000,0x2aaaaaaaaaaaaaaa};
    FqRawElement result161 = {0x0000000000000000,0x0000000000000000,0x5555555400000000,0x1555555555555555};

    FqRawElement result191 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555555555555};
    FqRawElement result192 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2aaaaaaaaaaaaaaa};
    FqRawElement result193 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555555555554};

    FqRawElement result223 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555500000000};
    FqRawElement result224 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2aaaaaaa00000000};
    FqRawElement result225 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x1555555400000000};

    FqRawElement result250 = {0xc3df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0b9bb18d1ece5fd6};
    FqRawElement result251 = {0xc3df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x079bb18d1ece5fd6};
    FqRawElement result252 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x3000000000000000};
    FqRawElement result253 = {0x0000000000000000,0x0000000000000000,0x0000000000000000,0x2000000000000000};

    Fq_rawShl_test(result1, rawA1, 1);
    Fq_rawShl_test(result2, rawA1, 2);
    Fq_rawShl_test(result3, rawA1, 3);
    Fq_rawShl_test(result4, rawA1, 4);

    Fq_rawShl_test(result7, rawA1, 7);
    Fq_rawShl_test(result8, rawA1, 8);
    Fq_rawShl_test(result9, rawA1, 9);

    Fq_rawShl_test(result15, rawA1, 15);
    Fq_rawShl_test(result16, rawA1, 16);
    Fq_rawShl_test(result17, rawA1, 17);

    Fq_rawShl_test(result30, rawA1, 30);
    Fq_rawShl_test(result31, rawA1, 31);
    Fq_rawShl_test(result32, rawA1, 32);
    Fq_rawShl_test(result33, rawA1, 33);
    Fq_rawShl_test(result34, rawA1, 34);

    Fq_rawShl_test(result63, rawA1, 63);
    Fq_rawShl_test(result64, rawA1, 64);
    Fq_rawShl_test(result65, rawA1, 65);

    Fq_rawShl_test(result95, rawA1, 95);
    Fq_rawShl_test(result96, rawA1, 96);
    Fq_rawShl_test(result97, rawA1, 97);

    Fq_rawShl_test(result127, rawA1, 127);
    Fq_rawShl_test(result128, rawA1, 128);
    Fq_rawShl_test(result129, rawA1, 129);

    Fq_rawShl_test(result159, rawA2, 159);
    Fq_rawShl_test(result160, rawA2, 160);
    Fq_rawShl_test(result161, rawA2, 161);

    Fq_rawShl_test(result191, rawA2, 191);
    Fq_rawShl_test(result192, rawA2, 192);
    Fq_rawShl_test(result193, rawA2, 193);

    Fq_rawShl_test(result223, rawA2, 223);
    Fq_rawShl_test(result224, rawA2, 224);
    Fq_rawShl_test(result225, rawA2, 225);

    Fq_rawShl_test(result250, rawA1, 250);
    Fq_rawShl_test(result251, rawA1, 251);
    Fq_rawShl_test(result252, rawA1, 252);
    Fq_rawShl_test(result253, rawA1, 253);
}

void Fr_square_test(FrElement r_expected, FrElement a, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_square(&r_computed, &a);

    compare_Result(&r_expected, &r_computed, &a, index, __func__);
}

void Fr_square_short_test(int64_t r_expected, int32_t a, int index)
{
    Fr_square_test(fr_long(r_expected), fr_short(a), index);
}

void Fr_square_unit_test()
{
    Fr_square_short_test(0,                0, 0);
    Fr_square_short_test(1,                1, 1);
    Fr_square_short_test(1,               -1, 2);
    Fr_square_short_test(4,                2, 3);
    Fr_square_short_test(4,               -2, 4);
    Fr_square_short_test(65536,          256, 5);
    Fr_square_short_test(65536,         -256, 6);
    Fr_square_short_test(1067851684,   32678, 7);
    Fr_square_short_test(4294967296,   65536, 8);
    Fr_square_short_test(68719476736, 262144, 9);

    FrElement a1 = fr_short(1048576);
    FrElement a2 = fr_short(16777216);
    FrElement a3 = fr_short(-16777216);
    FrElement a4 = fr_short(2147483647);
    FrElement a5 = fr_short(-2147483647);

    FrElement r1 = fr_long(0x0000010000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r2 = fr_long(0x0001000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r3 = fr_long(0x0001000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r4 = fr_long(0x3fffffff00000001,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r5 = fr_long(0x3fffffff00000001,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fr_square_test(r1, a1, 11);
    Fr_square_test(r2, a2, 12);
    Fr_square_test(r3, a3, 13);
    Fr_square_test(r4, a4, 14);
    Fr_square_test(r5, a5, 15);

    FrElement a21 = fr_long(0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement a22 = fr_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5);
    FrElement a23 = fr_long(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FrElement a24 = fr_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement a25 = fr_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);
    FrElement a26 = fr_long(0x1bb8e645ae216da7);

    FrElement r21 = fr_mlong(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r22 = fr_mlong(0x00915951a17a2cef,0xbf25f2dd9fd7425c,0xfb6cfdc4a7eeefb8,0x06eaaa4fb32c8ec9);
    FrElement r23 = fr_mlong(0xbd21a87879979b42,0xc838a7401d9b5225,0x97846f8ea771a174,0x00ae773b6f7fa82d);
    FrElement r24 = fr_mlong(0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r25 = fr_mlong(0x00915951a17a2cef,0xbf25f2dd9fd7425c,0xfb6cfdc4a7eeefb8,0x06eaaa4fb32c8ec9);
    FrElement r26 = fr_mlong(0x907220cfe9de6aa5,0xcbe953472316eb2c,0x2336c1a61ae5f272,0x136f2bc2b41ee96e);

    Fr_square_test(r21, a21, 21);
    Fr_square_test(r22, a22, 22);
    Fr_square_test(r23, a23, 23);
    Fr_square_test(r24, a24, 24);
    Fr_square_test(r25, a25, 25);
    Fr_square_test(r26, a26, 26);

    FrElement a31 = fr_mlong(0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement a32 = fr_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5);
    FrElement a33 = fr_mlong(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FrElement a34 = fr_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FrElement a35 = fr_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);
    FrElement a36 = fr_mlong(0x1bb8e645ae216da7);

    FrElement r31 = fr_mlong(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r32 = fr_mlong(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FrElement r33 = fr_mlong(0x00915951a17a2cef,0xbf25f2dd9fd7425c,0xfb6cfdc4a7eeefb8,0x06eaaa4fb32c8ec9);
    FrElement r34 = fr_mlong(0x9907e2cb536c4654,0xd65db18eb521336a,0x0e31a6546c6ec385,0x1dad258dd14a255c);
    FrElement r35 = fr_mlong(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FrElement r36 = fr_mlong(0xa53f1bf76b3483d6,0x368cb00a6a77e255,0x7b8b05c69920615c,0x0248823bc34637b8);

    Fr_square_test(r31, a31, 31);
    Fr_square_test(r32, a32, 32);
    Fr_square_test(r33, a33, 33);
    Fr_square_test(r34, a34, 34);
    Fr_square_test(r35, a35, 35);
    Fr_square_test(r36, a36, 36);
}


void Fq_square_test(FqElement r_expected, FqElement a, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_square(&r_computed, &a);

    compare_Result(&r_expected, &r_computed, &a, index, __func__);
}

void Fq_square_short_test(int64_t r_expected, int32_t a, int index)
{
    Fq_square_test(fq_long(r_expected), fq_short(a), index);
}

void Fq_square_unit_test()
{
    Fq_square_short_test(0,                0, 0);
    Fq_square_short_test(1,                1, 1);
    Fq_square_short_test(1,               -1, 2);
    Fq_square_short_test(4,                2, 3);
    Fq_square_short_test(4,               -2, 4);
    Fq_square_short_test(65536,          256, 5);
    Fq_square_short_test(65536,         -256, 6);
    Fq_square_short_test(1067851684,   32678, 7);
    Fq_square_short_test(4294967296,   65536, 8);
    Fq_square_short_test(68719476736, 262144, 9);

    FqElement a1 = fq_short(1048576);
    FqElement a2 = fq_short(16777216);
    FqElement a3 = fq_short(-16777216);
    FqElement a4 = fq_short(2147483647);
    FqElement a5 = fq_short(-2147483647);

    FqElement r1 = fq_long(0x0000010000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r2 = fq_long(0x0001000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r3 = fq_long(0x0001000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r4 = fq_long(0x3fffffff00000001,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r5 = fq_long(0x3fffffff00000001,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fq_square_test(r1, a1, 11);
    Fq_square_test(r2, a2, 12);
    Fq_square_test(r3, a3, 13);
    Fq_square_test(r4, a4, 14);
    Fq_square_test(r5, a5, 15);

    FqElement a21 = fq_long(0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FqElement a22 = fq_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5);
    FqElement a23 = fq_long(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FqElement a24 = fq_long(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement a25 = fq_long(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);
    FqElement a26 = fq_long(0x1bb8e645ae216da7);

    FqElement r21 = fq_mlong(0xa36e3db5ee5759d2,0x38352a2f67411193,0x827c17423bfb203b,0x2429ad301b3328c5);
    FqElement r22 = fq_mlong(0xcbe13c2bfb664022,0x50f6618240404b24,0xdac1e4a17673233b,0x0583edb5fdd86f35);
    FqElement r23 = fq_mlong(0xa1a671141ea315b1,0x1254a305ec52f02b,0x5c4b5ed24a33f0e3,0x1d80794f124ebcea);
    FqElement r24 = fq_mlong(0x58866a06a6cf3ccd,0xe7675ddd29531728,0xbca78e187e5fec64,0x05aaaec9bf8478e8);
    FqElement r25 = fq_mlong(0xcbe13c2bfb664022,0x50f6618240404b24,0xdac1e4a17673233b,0x0583edb5fdd86f35);
    FqElement r26 = fq_mlong(0xbd7c163fbc00a4c3,0xb02513c97a803400,0x1a4492de859a2863,0x0c878a77effa01c6);

    Fq_square_test(r21, a21, 21);
    Fq_square_test(r22, a22, 22);
    Fq_square_test(r23, a23, 23);
    Fq_square_test(r24, a24, 24);
    Fq_square_test(r25, a25, 25);
    Fq_square_test(r26, a26, 26);

    FqElement a31 = fq_mlong(0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FqElement a32 = fq_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5);
    FqElement a33 = fq_mlong(0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c);
    FqElement a34 = fq_mlong(0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014);
    FqElement a35 = fq_mlong(0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x216d0b17f4e44a5);
    FqElement a36 = fq_mlong(0x1bb8e645ae216da7);

    FqElement r31 = fq_mlong(0x355fdbd1472c705a,0x4cc7e466a7fbb77f,0x8658fb1c77f4a809,0x23aef213fb88c295);
    FqElement r32 = fq_mlong(0x1fa2e058e64e824a,0x053324c431844d78,0x4bf3dac062ea6dad,0x2db3e562977df94a);
    FqElement r33 = fq_mlong(0x644c5ce20a8793bb,0xebc09ef48a61c906,0x0281385bd1007d0c,0x1bce0f38b8cdaad9);
    FqElement r34 = fq_mlong(0xac0b6f222f67487d,0x364d764ea56127d9,0xe5ad1f8aa6ef1ae1,0x2dffef30a4034c35);
    FqElement r35 = fq_mlong(0x1fa2e058e64e824a,0x053324c431844d78,0x4bf3dac062ea6dad,0x2db3e562977df94a);
    FqElement r36 = fq_mlong(0x49d481ec59aa5401,0x804ca61c080d6da3,0x4e6b2f7e337fa8d1,0x0f2dcfc4e7661f81);

    Fq_square_test(r31, a31, 31);
    Fq_square_test(r32, a32, 32);
    Fq_square_test(r33, a33, 33);
    Fq_square_test(r34, a34, 34);
    Fq_square_test(r35, a35, 35);
    Fq_square_test(r36, a36, 36);
}

void Fr_bor_test(FrElement r_expected, FrElement a, FrElement b, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_bor(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fr_bor_unit_test()
{
    FrElement s0  = fr_short(0);
    FrElement sf  = fr_short(0x7fffffff);
    FrElement s5  = fr_short(0x55555555);
    FrElement s9  = fr_short(0x99999999);
    FrElement sf1 = fr_short(-1);
    FrElement sf5 = fr_short(0xf5555555);
    FrElement sf9 = fr_short(0xf9999999);

    FrElement r2 = fr_long(0x43e1f5938999999a,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r3 = fr_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r4 = fr_long(0x43e1f593e5555556,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r5 = fr_long(0x43e1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);

    FrElement r12 = fr_long(0x43e1f593dddddddf,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r13 = fr_long(0x000000000ffffffe,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r14 = fr_long(0x43e1f593dddddddf,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r15 = fr_long(0x000000000ffffffe,0x0000000000000000,0x0000000000000000,0x0000000000000000);


    Fr_bor_test(sf, s0, sf,  0);
    Fr_bor_test(s5, s0, s5,  1);
    Fr_bor_test(r2, s0, s9,  2);
    Fr_bor_test(r3, s0, sf1, 3);
    Fr_bor_test(r4, s0, sf5, 4);
    Fr_bor_test(r5, s0, sf9, 5);

    Fr_bor_test(sf, sf,  s0, 6);
    Fr_bor_test(s5, s5,  s0, 7);
    Fr_bor_test(r2, s9,  s0, 8);
    Fr_bor_test(r3, sf1, s0, 9);
    Fr_bor_test(r4, sf5, s0, 10);
    Fr_bor_test(r5, sf9, s0, 11);

    Fr_bor_test(r12, s5,  s9, 12);
    Fr_bor_test(r13, sf1, sf, 13);
    Fr_bor_test(r14, s9,  s5, 14);
    Fr_bor_test(r15, sf, sf1, 15);

    FrElement l0 = fr_long(0);
    FrElement l1 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement l2 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0xf9999999);
    FrElement l5 = fr_long(0xf5555555);
    FrElement l9 = fr_long(0xf9999999);

    FrElement r21 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r22 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r23 = fr_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r24 = fr_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r25 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r26 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r27 = fr_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r28 = fr_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r29 = fr_long(0x43e1f593f5555555,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r30 = fr_long(0x43e1f593f9999999,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r31 = fr_long(0xbc0000000999999a,0x04c811030644056c,0x0000000000000000,0x0000000018881990);
    FrElement r32 = fr_long(0xffe1f593fddddddf,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r33 = fr_long(0xffe1f593f999999b,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r34 = fr_long(0xbc0000000999999a,0x04c811030644056c,0x0000000000000000,0x0000000018881990);
    FrElement r35 = fr_long(0x00000000fddddddd,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fr_bor_test(r21, l0, l1, 21);
    Fr_bor_test(r22, l0, l2, 22);
    Fr_bor_test(r23, l0, l5, 23);
    Fr_bor_test(r24, l0, l9, 24);
    Fr_bor_test(r25, l1, l0, 25);
    Fr_bor_test(r26, l2, l0, 26);
    Fr_bor_test(r27, l5, l0, 27);
    Fr_bor_test(r28, l9, l0, 28);
    Fr_bor_test(r29, l1, l5, 29);
    Fr_bor_test(r30, l1, l9, 30);
    Fr_bor_test(r31, l1, l2, 31);
    Fr_bor_test(r32, l2, l5, 32);
    Fr_bor_test(r33, l2, l9, 33);
    Fr_bor_test(r34, l2, l1, 34);
    Fr_bor_test(r35, l5, l9, 35);

    FrElement m0 = fr_mlong(0);
    FrElement m1 = fr_mlong(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement m5 = fr_mlong(0xf5555555);

    FrElement r41 = fr_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r42 = fr_long(0x7385aa3557a85e96,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r43 = fr_long(0x6656931836f71fc0,0xd91d972332e0fff9,0x6d1dc7a7d4dfb843,0x1151f9979bbe9426);
    FrElement r44 = fr_long(0x33f5c5a987ff5fd5,0xb10a0f1b41458f6e,0xc56f8209757e64a2,0x059bb144ba8d5ebd);
    FrElement r45 = fr_long(0x33f5c5a987ff5fd5,0xb10a0f1b41458f6e,0xc56f8209757e64a2,0x059bb144ba8d5ebd);
    FrElement r46 = fr_long(0x6656931836f71fc0,0xd91d972332e0fff9,0x6d1dc7a7d4dfb843,0x1151f9979bbe9426);

    Fr_bor_test(r41, m0, m0, 41);
    Fr_bor_test(r42, m0, m1, 42);
    Fr_bor_test(r43, m0, m5, 43);
    Fr_bor_test(r44, m1, m5, 44);
    Fr_bor_test(r45, m5, m1, 45);
    Fr_bor_test(r46, m5, m0, 46);


    FrElement r51 = fr_long(0x30040a23efb9df9d,0x110c16038006820e,0x44a38209262c84a2,0x048a21050a0c5ac0);
    FrElement r52 = fr_long(0xbbfffffff9999999,0x0000000000000000,0x0000000000000000,0x0000000018881990);
    FrElement r53 = fr_long(0x7385aa357fffffff,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r54 = fr_long(0xffe1f593ffffffff,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r55 = fr_long(0xffe5ffb7ffb9df9e,0x393ffe4bf9bff29f,0xfcf3c7bfa7addcff,0x24ee2725fbbdfbd9);
    FrElement r56 = fr_long(0xffe5ffb7ffb9df9e,0x393ffe4bf9bff29f,0xfcf3c7bfa7addcff,0x24ee2725fbbdfbd9);
    FrElement r57 = fr_long(0x30040a23efb9df9d,0x110c16038006820e,0x44a38209262c84a2,0x048a21050a0c5ac0);
    FrElement r58 = fr_long(0xbbfffffff9999999,0x0000000000000000,0x0000000000000000,0x0000000018881990);
    FrElement r59 = fr_long(0x7385aa357fffffff,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r50 = fr_long(0xffe1f593ffffffff,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);

    Fr_bor_test(r51, s9, m1, 51);
    Fr_bor_test(r52, s9, l2, 52);
    Fr_bor_test(r53, sf, m1, 53);
    Fr_bor_test(r54, sf, l2, 54);
    Fr_bor_test(r55, l2, m1, 55);
    Fr_bor_test(r56, m1, l2, 56);
    Fr_bor_test(r57, m1, s9, 57);
    Fr_bor_test(r58, l2, s9, 58);
    Fr_bor_test(r59, m1, sf, 59);
    Fr_bor_test(r50, l2, sf, 50);
}

void Fr_bxor_test(FrElement r_expected, FrElement a, FrElement b, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_bxor(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fr_bxor_unit_test()
{
    FrElement s0  = fr_short(0);
    FrElement sf  = fr_short(0x7fffffff);
    FrElement s5  = fr_short(0x55555555);
    FrElement s9  = fr_short(0x99999999);
    FrElement sf1 = fr_short(-1);
    FrElement sf5 = fr_short(0xf5555555);
    FrElement sf9 = fr_short(0xf9999999);

    FrElement r2 = fr_long(0x43e1f5938999999a,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r3 = fr_long(0x43e1f593f0000000,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r4 = fr_long(0x43e1f593e5555556,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r5 = fr_long(0x43e1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);

    FrElement r12 = fr_long(0x43e1f593dccccccf,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r13 = fr_long(0x43e1f5938fffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r14 = fr_long(0x43e1f593dccccccf,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r15 = fr_long(0x43e1f5938fffffff,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029);


    Fr_bxor_test(sf, s0, sf,  0);
    Fr_bxor_test(s5, s0, s5,  1);
    Fr_bxor_test(r2, s0, s9,  2);
    Fr_bxor_test(r3, s0, sf1, 3);
    Fr_bxor_test(r4, s0, sf5, 4);
    Fr_bxor_test(r5, s0, sf9, 5);

    Fr_bxor_test(sf, sf,  s0, 6);
    Fr_bxor_test(s5, s5,  s0, 7);
    Fr_bxor_test(r2, s9,  s0, 8);
    Fr_bxor_test(r3, sf1, s0, 9);
    Fr_bxor_test(r4, sf5, s0, 10);
    Fr_bxor_test(r5, sf9, s0, 11);

    Fr_bxor_test(r12, s5,  s9, 12);
    Fr_bxor_test(r13, sf1, sf, 13);
    Fr_bxor_test(r14, s9,  s5, 14);
    Fr_bxor_test(r15, sf, sf1, 15);

    FrElement l0 = fr_long(0);
    FrElement l1 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement l2 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0xf9999999);
    FrElement l5 = fr_long(0xf5555555);
    FrElement l9 = fr_long(0xf9999999);

    FrElement r21 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r22 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r23 = fr_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r24 = fr_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r25 = fr_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r26 = fr_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r27 = fr_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r28 = fr_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r29 = fr_long(0x43e1f59305555554,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r30 = fr_long(0x43e1f59309999998,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement r31 = fr_long(0xbc0000001999999b,0x24cbb103067515ed,0x0000000000000000,0x30644e7218a839b0);
    FrElement r32 = fr_long(0xffe1f5931ccccccf,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r33 = fr_long(0xffe1f59310000003,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r34 = fr_long(0xbc0000001999999b,0x24cbb103067515ed,0x0000000000000000,0x30644e7218a839b0);
    FrElement r35 = fr_long(0x000000000ccccccc,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fr_bxor_test(r21, l0, l1, 21);
    Fr_bxor_test(r22, l0, l2, 22);
    Fr_bxor_test(r23, l0, l5, 23);
    Fr_bxor_test(r24, l0, l9, 24);
    Fr_bxor_test(r25, l1, l0, 25);
    Fr_bxor_test(r26, l2, l0, 26);
    Fr_bxor_test(r27, l5, l0, 27);
    Fr_bxor_test(r28, l9, l0, 28);
    Fr_bxor_test(r29, l1, l5, 29);
    Fr_bxor_test(r30, l1, l9, 30);
    Fr_bxor_test(r31, l1, l2, 31);
    Fr_bxor_test(r32, l2, l5, 32);
    Fr_bxor_test(r33, l2, l9, 33);
    Fr_bxor_test(r34, l2, l1, 34);
    Fr_bxor_test(r35, l5, l9, 35);

    FrElement m0 = fr_mlong(0);
    FrElement m1 = fr_mlong(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FrElement m5 = fr_mlong(0xf5555555);

    FrElement r41 = fr_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r42 = fr_long(0x7385aa3557a85e96,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r43 = fr_long(0x6656931836f71fc0,0xd91d972332e0fff9,0x6d1dc7a7d4dfb843,0x1151f9979bbe9426);
    FrElement r44 = fr_long(0xd1f14399715f4155,0x97fd791840a4ed55,0x596e000470f0cc60,0x055b903fb060cebd);
    FrElement r45 = fr_long(0xd1f14399715f4155,0x97fd791840a4ed55,0x596e000470f0cc60,0x055b903fb060cebd);
    FrElement r46 = fr_long(0x6656931836f71fc0,0xd91d972332e0fff9,0x6d1dc7a7d4dfb843,0x1151f9979bbe9426);

    Fr_bxor_test(r41, m0, m0, 41);
    Fr_bxor_test(r42, m0, m1, 42);
    Fr_bxor_test(r43, m0, m5, 43);
    Fr_bxor_test(r44, m1, m5, 44);
    Fr_bxor_test(r45, m5, m1, 45);
    Fr_bxor_test(r46, m5, m0, 46);


    FrElement r51 = fr_long(0x30645fa6de31c70c,0x311f1e0bf107d28f,0xc4f3c7aba72cc4a3,0x148a6957eb1d5ae8);
    FrElement r52 = fr_long(0xbc00000060000000,0x0000000000000000,0x0000000000000000,0x30644e7218a839b0);
    FrElement r53 = fr_long(0x7385aa352857a169,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r54 = fr_long(0xffe1f59396666665,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FrElement r55 = fr_long(0x8c645fa6be31c70c,0x311f1e0bf107d28f,0xc4f3c7aba72cc4a3,0x24ee2725f3b56358);
    FrElement r56 = fr_long(0x8c645fa6be31c70c,0x311f1e0bf107d28f,0xc4f3c7aba72cc4a3,0x24ee2725f3b56358);
    FrElement r57 = fr_long(0x30645fa6de31c70c,0x311f1e0bf107d28f,0xc4f3c7aba72cc4a3,0x148a6957eb1d5ae8);
    FrElement r58 = fr_long(0xbc00000060000000,0x0000000000000000,0x0000000000000000,0x30644e7218a839b0);
    FrElement r59 = fr_long(0x7385aa352857a169,0x192cf64388bea21e,0x7ca3821d26ad9cfe,0x24ee27250a2cfac1);
    FrElement r50 = fr_long(0xffe1f59396666665,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);

    Fr_bxor_test(r51, s9, m1, 51);
    Fr_bxor_test(r52, s9, l2, 52);
    Fr_bxor_test(r53, sf, m1, 53);
    Fr_bxor_test(r54, sf, l2, 54);
    Fr_bxor_test(r55, l2, m1, 55);
    Fr_bxor_test(r56, m1, l2, 56);
    Fr_bxor_test(r57, m1, s9, 57);
    Fr_bxor_test(r58, l2, s9, 58);
    Fr_bxor_test(r59, m1, sf, 59);
    Fr_bxor_test(r50, l2, sf, 50);
}


void Fr_bnot_test(FrElement r_expected, FrElement a, int index)
{
    FrElement r_computed = {0,0,{0,0,0,0}};

    Fr_bnot(&r_computed, &a);

    compare_Result(&r_expected, &r_computed, &a, index, __func__);
}

void Fr_bnot_unit_test()
{
    FrElement s0  = fr_short(0);
    FrElement s1  = fr_short(0x7fffffff);
    FrElement s2  = fr_short(0xffffffff);
    FrElement s3  = fr_short(0x55555555);
    FrElement s4  = fr_short(0x99999999);

    FrElement r0 = fr_long(0xbc1e0a6c0ffffffe,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r1 = fr_long(0xbc1e0a6b8fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r2 = fr_long(0xbc1e0a6c0fffffff,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r3 = fr_long(0xbc1e0a6bbaaaaaa9,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r4 = fr_long(0xbc1e0a6c76666665,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);

    Fr_bnot_test(r0, s0, 0);
    Fr_bnot_test(r1, s1, 1);
    Fr_bnot_test(r2, s2, 2);
    Fr_bnot_test(r3, s3, 3);
    Fr_bnot_test(r4, s4, 4);


    FrElement l0 = fr_long(0);
    FrElement l1 = fr_long(0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff);
    FrElement l2 = fr_long(0x5555555555555555,0x5555555555555555,0x5555555555555555,0x5555555555555555);
    FrElement l3 = fr_long(0x9999999999999999,0x9999999999999999,0x9999999999999999,0x9999999999999999);

    FrElement r10 = fr_long(0xbc1e0a6c0ffffffe,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r11 = fr_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FrElement r12 = fr_long(0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0x2aaaaaaaaaaaaaaa);
    FrElement r13 = fr_long(0x6666666666666666,0x6666666666666666,0x6666666666666666,0x2666666666666666);

    Fr_bnot_test(r10, l0, 10);
    Fr_bnot_test(r11, l1, 11);
    Fr_bnot_test(r12, l2, 12);
    Fr_bnot_test(r13, l3, 13);

    FrElement m0 = fr_mlong(0);
    FrElement m1 = fr_mlong(0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff);
    FrElement m2 = fr_mlong(0x5555555555555555,0x5555555555555555,0x5555555555555555,0x5555555555555555);
    FrElement m3 = fr_mlong(0x9999999999999999,0x9999999999999999,0x9999999999999999,0x9999999999999999);

    FrElement r20 = fr_long(0xbc1e0a6c0ffffffe,0xd7cc17b786468f6e,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FrElement r21 = fr_long(0x9879aa717db1194b,0xe0db0d6167587bf6,0x0fd5c82e2d3704ff,0x2587aadea193b4f3);
    FrElement r22 = fr_long(0x0591ea6ddf3b086d,0xdad114457bf7339c,0x8a6714406366c6c1,0x16ea59fd9fbad18a);
    FrElement r23 = fr_long(0xbec76e9a8b6a425f,0x99f38166dca0bd1f,0x0fa67389b38655e8,0x09678e29acca860a);

    Fr_bnot_test(r20, m0, 20);
    Fr_bnot_test(r21, m1, 21);
    Fr_bnot_test(r22, m2, 22);
    Fr_bnot_test(r23, m3, 23);
}

void Fq_bor_test(FqElement r_expected, FqElement a, FqElement b, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_bor(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fq_bor_unit_test()
{
    FqElement s0  = fq_short(0);
    FqElement sf  = fq_short(0x7fffffff);
    FqElement s5  = fq_short(0x55555555);
    FqElement s9  = fq_short(0x99999999);
    FqElement sf1 = fq_short(-1);
    FqElement sf5 = fq_short(0xf5555555);
    FqElement sf9 = fq_short(0xf9999999);

    FqElement r2 = fq_long(0x3c208c16721696e0,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r3 = fq_long(0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r4 = fq_long(0x3c208c16cdd2529c,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r5 = fq_long(0x3c208c16d21696e0,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);

    FqElement r12 = fq_long(0x3c208c167757d7f5,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r13 = fq_long(0x00000000278302b8,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r14 = fq_long(0x3c208c167757d7f5,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r15 = fq_long(0x00000000278302b8,0x0000000000000000,0x0000000000000000,0x0000000000000000);


    Fq_bor_test(sf, s0, sf,  0);
    Fq_bor_test(s5, s0, s5,  1);
    Fq_bor_test(r2, s0, s9,  2);
    Fq_bor_test(r3, s0, sf1, 3);
    Fq_bor_test(r4, s0, sf5, 4);
    Fq_bor_test(r5, s0, sf9, 5);

    Fq_bor_test(sf, sf,  s0, 6);
    Fq_bor_test(s5, s5,  s0, 7);
    Fq_bor_test(r2, s9,  s0, 8);
    Fq_bor_test(r3, sf1, s0, 9);
    Fq_bor_test(r4, sf5, s0, 10);
    Fq_bor_test(r5, sf9, s0, 11);

    Fq_bor_test(r12, s5,  s9, 12);
    Fq_bor_test(r13, sf1, sf, 13);
    Fq_bor_test(r14, s9,  s5, 14);
    Fq_bor_test(r15, sf, sf1, 15);

    FqElement l0 = fq_long(0);
    FqElement l1 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement l2 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0xf9999999);
    FqElement l5 = fq_long(0xf5555555);
    FqElement l9 = fq_long(0xf9999999);

    FqElement r21 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r22 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r23 = fq_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r24 = fq_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r25 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r26 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r27 = fq_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r28 = fq_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r29 = fq_long(0x43e1f593f5555555,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r30 = fq_long(0x43e1f593f9999999,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r31 = fq_long(0xc3c1697d211c9c54,0x957a8eba178bab70,0xffffffffffffffff,0x000000001888198f);
    FqElement r32 = fq_long(0xffe1f593fddddddf,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r33 = fq_long(0xffe1f593f999999b,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r34 = fq_long(0xc3c1697d211c9c54,0x957a8eba178bab70,0xffffffffffffffff,0x000000001888198f);
    FqElement r35 = fq_long(0x00000000fddddddd,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fq_bor_test(r21, l0, l1, 21);
    Fq_bor_test(r22, l0, l2, 22);
    Fq_bor_test(r23, l0, l5, 23);
    Fq_bor_test(r24, l0, l9, 24);
    Fq_bor_test(r25, l1, l0, 25);
    Fq_bor_test(r26, l2, l0, 26);
    Fq_bor_test(r27, l5, l0, 27);
    Fq_bor_test(r28, l9, l0, 28);
    Fq_bor_test(r29, l1, l5, 29);
    Fq_bor_test(r30, l1, l9, 30);
    Fq_bor_test(r31, l1, l2, 31);
    Fq_bor_test(r32, l2, l5, 32);
    Fq_bor_test(r33, l2, l9, 33);
    Fq_bor_test(r34, l2, l1, 34);
    Fq_bor_test(r35, l5, l9, 35);

    FqElement m0 = fq_mlong(0);
    FqElement m1 = fq_mlong(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement m5 = fq_mlong(0xf5555555);

    FqElement r41 = fq_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r42 = fq_long(0xd0efff77d802b158,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r43 = fq_long(0x308a7b727182808d,0xa4628937feb96fa3,0xd285c952692a2871,0x0ce9ab0ad29a701c);
    FqElement r44 = fq_long(0xf0efff77f982b1dd,0xae63a9bffefb6fab,0xdbaddd52ffeae8f1,0x1de9bfabfb9ff13f);
    FqElement r45 = fq_long(0xf0efff77f982b1dd,0xae63a9bffefb6fab,0xdbaddd52ffeae8f1,0x1de9bfabfb9ff13f);
    FqElement r46 = fq_long(0x308a7b727182808d,0xa4628937feb96fa3,0xd285c952692a2871,0x0ce9ab0ad29a701c);

    Fq_bor_test(r41, m0, m0, 41);
    Fq_bor_test(r42, m0, m1, 42);
    Fq_bor_test(r43, m0, m5, 43);
    Fq_bor_test(r44, m1, m5, 44);
    Fq_bor_test(r45, m5, m1, 45);
    Fq_bor_test(r46, m5, m0, 46);


    FqElement r51 = fq_long(0xc0cf73612199bab1,0x2802010a92822520,0x01ac90001648a0a0,0x0908108908840106);
    FqElement r52 = fq_long(0xc3c171812322a2b3,0x2832804811883010,0x0000000000000000,0x0000000018881990);
    FqElement r53 = fq_long(0xd0efff77ffffffff,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r54 = fq_long(0xffe1f593ffffffff,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r55 = fq_long(0xffeffff7f99bb9da,0xae33e9dafbfb75b9,0xb9fcd5b697c9f8fd,0x190816a9f99db9bf);
    FqElement r56 = fq_long(0xffeffff7f99bb9da,0xae33e9dafbfb75b9,0xb9fcd5b697c9f8fd,0x190816a9f99db9bf);
    FqElement r57 = fq_long(0xc0cf73612199bab1,0x2802010a92822520,0x01ac90001648a0a0,0x0908108908840106);
    FqElement r58 = fq_long(0xc3c171812322a2b3,0x2832804811883010,0x0000000000000000,0x0000000018881990);
    FqElement r59 = fq_long(0xd0efff77ffffffff,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r50 = fq_long(0xffe1f593ffffffff,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);

    Fq_bor_test(r51, s9, m1, 51);
    Fq_bor_test(r52, s9, l2, 52);
    Fq_bor_test(r53, sf, m1, 53);
    Fq_bor_test(r54, sf, l2, 54);
    Fq_bor_test(r55, l2, m1, 55);
    Fq_bor_test(r56, m1, l2, 56);
    Fq_bor_test(r57, m1, s9, 57);
    Fq_bor_test(r58, l2, s9, 58);
    Fq_bor_test(r59, m1, sf, 59);
    Fq_bor_test(r50, l2, sf, 50);
}

void Fq_bxor_test(FqElement r_expected, FqElement a, FqElement b, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_bxor(&r_computed, &a, &b);

    compare_Result(&r_expected, &r_computed, &a, &b, index, __func__);
}

void Fq_bxor_unit_test()
{
    FqElement s0  = fq_short(0);
    FqElement sf  = fq_short(0x7fffffff);
    FqElement s5  = fq_short(0x55555555);
    FqElement s9  = fq_short(0x99999999);
    FqElement sf1 = fq_short(-1);
    FqElement sf5 = fq_short(0xf5555555);
    FqElement sf9 = fq_short(0xf9999999);

    FqElement r2 = fq_long(0x3c208c16721696e0,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r3 = fq_long(0x3c208c16d87cfd46,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r4 = fq_long(0x3c208c16cdd2529c,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r5 = fq_long(0x3c208c16d21696e0,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);

    FqElement r12 = fq_long(0x3c208c162743c3b5,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r13 = fq_long(0x3c208c16a78302b9,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r14 = fq_long(0x3c208c162743c3b5,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r15 = fq_long(0x3c208c16a78302b9,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029);


    Fq_bxor_test(sf, s0, sf,  0);
    Fq_bxor_test(s5, s0, s5,  1);
    Fq_bxor_test(r2, s0, s9,  2);
    Fq_bxor_test(r3, s0, sf1, 3);
    Fq_bxor_test(r4, s0, sf5, 4);
    Fq_bxor_test(r5, s0, sf9, 5);

    Fq_bxor_test(sf, sf,  s0, 6);
    Fq_bxor_test(s5, s5,  s0, 7);
    Fq_bxor_test(r2, s9,  s0, 8);
    Fq_bxor_test(r3, sf1, s0, 9);
    Fq_bxor_test(r4, sf5, s0, 10);
    Fq_bxor_test(r5, sf9, s0, 11);

    Fq_bxor_test(r12, s5,  s9, 12);
    Fq_bxor_test(r13, sf1, sf, 13);
    Fq_bxor_test(r14, s9,  s5, 14);
    Fq_bxor_test(r15, sf, sf1, 15);

    FqElement l0 = fq_long(0);
    FqElement l1 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement l2 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0xf9999999);
    FqElement l5 = fq_long(0xf5555555);
    FqElement l9 = fq_long(0xf9999999);

    FqElement r21 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r22 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r23 = fq_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r24 = fq_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r25 = fq_long(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r26 = fq_long(0xffe1f593e999999a,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r27 = fq_long(0x00000000f5555555,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r28 = fq_long(0x00000000f9999999,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r29 = fq_long(0x43e1f59305555554,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r30 = fq_long(0x43e1f59309999998,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement r31 = fq_long(0xbc0000001999999b,0x24cbb103067515ed,0x0000000000000000,0x30644e7218a839b0);
    FqElement r32 = fq_long(0xffe1f5931ccccccf,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r33 = fq_long(0xffe1f59310000003,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r34 = fq_long(0xbc0000001999999b,0x24cbb103067515ed,0x0000000000000000,0x30644e7218a839b0);
    FqElement r35 = fq_long(0x000000000ccccccc,0x0000000000000000,0x0000000000000000,0x0000000000000000);

    Fq_bxor_test(r21, l0, l1, 21);
    Fq_bxor_test(r22, l0, l2, 22);
    Fq_bxor_test(r23, l0, l5, 23);
    Fq_bxor_test(r24, l0, l9, 24);
    Fq_bxor_test(r25, l1, l0, 25);
    Fq_bxor_test(r26, l2, l0, 26);
    Fq_bxor_test(r27, l5, l0, 27);
    Fq_bxor_test(r28, l9, l0, 28);
    Fq_bxor_test(r29, l1, l5, 29);
    Fq_bxor_test(r30, l1, l9, 30);
    Fq_bxor_test(r31, l1, l2, 31);
    Fq_bxor_test(r32, l2, l5, 32);
    Fq_bxor_test(r33, l2, l9, 33);
    Fq_bxor_test(r34, l2, l1, 34);
    Fq_bxor_test(r35, l5, l9, 35);

    FqElement m0 = fq_mlong(0);
    FqElement m1 = fq_mlong(0x43e1f593f0000001,0x0cf8594b7fcc657c,0xb85045b68181585d,0x30644e72e131a029);
    FqElement m5 = fq_mlong(0xf5555555);

    FqElement r41 = fq_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r42 = fq_long(0xd0efff77d802b158,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r43 = fq_long(0x308a7b727182808d,0xa4628937feb96fa3,0xd285c952692a2871,0x0ce9ab0ad29a701c);
    FqElement r44 = fq_long(0xe0658405a98031d5,0x0a61a0ad245b4a8a,0xdb291c50ffe2c881,0x15e1bda3fb1fd133);
    FqElement r45 = fq_long(0xe0658405a98031d5,0x0a61a0ad245b4a8a,0xdb291c50ffe2c881,0x15e1bda3fb1fd133);
    FqElement r46 = fq_long(0x308a7b727182808d,0xa4628937feb96fa3,0xd285c952692a2871,0x0ce9ab0ad29a701c);

    Fq_bxor_test(r41, m0, m0, 41);
    Fq_bxor_test(r42, m0, m1, 42);
    Fq_bxor_test(r43, m0, m5, 43);
    Fq_bxor_test(r44, m1, m5, 44);
    Fq_bxor_test(r45, m5, m1, 45);
    Fq_bxor_test(r46, m5, m0, 46);


    FqElement r51 = fq_long(0xeccf7361aa1427b8,0x3982430bb293efa4,0xb1fc90b41749b8ad,0x296c58dbc8b40106);
    FqElement r52 = fq_long(0xc3c179859b8f0f7a,0xbfb282d911c8ba1c,0x0000000000000000,0x30644e7218a839b0);
    FqElement r53 = fq_long(0xd0efff77a7fd4ea7,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r54 = fq_long(0xffe1f59396666665,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);
    FqElement r55 = fq_long(0x2f0e0ae4319b28c2,0x8630c1d2a35b55b8,0xb1fc90b41749b8ad,0x190816a9d01c38b6);
    FqElement r56 = fq_long(0x2f0e0ae4319b28c2,0x8630c1d2a35b55b8,0xb1fc90b41749b8ad,0x190816a9d01c38b6);
    FqElement r57 = fq_long(0xeccf7361aa1427b8,0x3982430bb293efa4,0xb1fc90b41749b8ad,0x296c58dbc8b40106);
    FqElement r58 = fq_long(0xc3c179859b8f0f7a,0xbfb282d911c8ba1c,0x0000000000000000,0x30644e7218a839b0);
    FqElement r59 = fq_long(0xd0efff77a7fd4ea7,0xae03299adae22529,0x09acd50296c8e0f0,0x190816a92985a12f);
    FqElement r50 = fq_long(0xffe1f59396666665,0x2833e84879b97091,0xb85045b68181585d,0x00000000f9999999);

    Fq_bxor_test(r51, s9, m1, 51);
    Fq_bxor_test(r52, s9, l2, 52);
    Fq_bxor_test(r53, sf, m1, 53);
    Fq_bxor_test(r54, sf, l2, 54);
    Fq_bxor_test(r55, l2, m1, 55);
    Fq_bxor_test(r56, m1, l2, 56);
    Fq_bxor_test(r57, m1, s9, 57);
    Fq_bxor_test(r58, l2, s9, 58);
    Fq_bxor_test(r59, m1, sf, 59);
    Fq_bxor_test(r50, l2, sf, 50);
}


void Fq_bnot_test(FqElement r_expected, FqElement a, int index)
{
    FqElement r_computed = {0,0,{0,0,0,0}};

    Fq_bnot(&r_computed, &a);

    compare_Result(&r_expected, &r_computed, &a, index, __func__);
}

void Fq_bnot_unit_test()
{
    FqElement s0  = fq_short(0);
    FqElement s1  = fq_short(0x7fffffff);
    FqElement s2  = fq_short(0xffffffff);
    FqElement s3  = fq_short(0x55555555);
    FqElement s4  = fq_short(0x99999999);

    FqElement r0 = fq_long(0xc3df73e9278302b8,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r1 = fq_long(0xc3df73e8a78302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r2 = fq_long(0xc3df73e9278302b9,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r3 = fq_long(0xc3df73e8d22dad63,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r4 = fq_long(0xc3df73e98de9691f,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);

    Fq_bnot_test(r0, s0, 0);
    Fq_bnot_test(r1, s1, 1);
    Fq_bnot_test(r2, s2, 2);
    Fq_bnot_test(r3, s3, 3);
    Fq_bnot_test(r4, s4, 4);


    FqElement l0 = fq_long(0);
    FqElement l1 = fq_long(0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff);
    FqElement l2 = fq_long(0x5555555555555555,0x5555555555555555,0x5555555555555555,0x5555555555555555);
    FqElement l3 = fq_long(0x9999999999999999,0x9999999999999999,0x9999999999999999,0x9999999999999999);

    FqElement r10 = fq_long(0xc3df73e9278302b8,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r11 = fq_long(0x0000000000000000,0x0000000000000000,0x0000000000000000,0x0000000000000000);
    FqElement r12 = fq_long(0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0xaaaaaaaaaaaaaaaa,0x2aaaaaaaaaaaaaaa);
    FqElement r13 = fq_long(0x6666666666666666,0x6666666666666666,0x6666666666666666,0x2666666666666666);

    Fq_bnot_test(r10, l0, 10);
    Fq_bnot_test(r11, l1, 11);
    Fq_bnot_test(r12, l2, 12);
    Fq_bnot_test(r13, l3, 13);

    FqElement m0 = fq_mlong(0);
    FqElement m1 = fq_mlong(0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff,0xffffffffffffffff);
    FqElement m2 = fq_mlong(0x5555555555555555,0x5555555555555555,0x5555555555555555,0x5555555555555555);
    FqElement m3 = fq_mlong(0x9999999999999999,0x9999999999999999,0x9999999999999999,0x9999999999999999);

    FqElement r20 = fq_long(0xc3df73e9278302b8,0x687e956e978e3572,0x47afba497e7ea7a2,0x0f9bb18d1ece5fd6);
    FqElement r21 = fq_long(0x7543701c5050ffa7,0xbc1d4d62319558de,0x5ec35e62b441e21e,0x0d9e788b978285e6);
    FqElement r22 = fq_long(0xbdb64c01d29c00ca,0x61894b9aedb684c5,0xe2265d8ebb958340,0x1f13635d921af73e);
    FqElement r23 = fq_long(0x85955f48c1e33272,0x8f2b76f19909f76e,0x5db879c61fa7cc8b,0x2b7324d1218b705f);

    Fq_bnot_test(r20, m0, 20);
    Fq_bnot_test(r21, m1, 21);
    Fq_bnot_test(r22, m2, 22);
    Fq_bnot_test(r23, m3, 23);
}

void print_results()
{
    std::cout << "Results: " << std::dec << tests_run << " tests were run, " << tests_failed << " failed." << std::endl;
}

int main()
{
    Fr_Rw_add_unit_test();
    Fr_Rw_sub_unit_test();
    Fr_Rw_copy_unit_test();
    Fr_Rw_Neg_unit_test();
    Fr_Rw_mul_unit_test();
    Fr_Rw_Msquare_unit_test();
    Fr_Rw_mul1_unit_test();
    Fr_Rw_ToMontgomery_unit_test();
    Fr_Rw_IsEq_unit_test();
    Fr_rawIsZero_unit_test();
    Fr_Rw_FromMontgomery_unit_test();
    Fr_toNormal_unit_test();
    Fr_copy_unit_test();
    Fr_copyn_unit_test();
    Fr_mul_s1s2_unit_test();
    Fr_mul_l1nl2n_unit_test();
    Fr_mul_l1ml2n_unit_test();
    Fr_mul_l1ml2m_unit_test();
    Fr_mul_l1nl2m_unit_test();
    Fr_mul_l1ns2n_unit_test();
    Fr_mul_s1nl2n_unit_test();
    Fr_mul_s1nl2m_unit_test();
    Fr_mul_l1ms2n_unit_test();
    Fr_mul_l1ns2m_unit_test();
    Fr_mul_l1ms2m_unit_test();
    Fr_mul_s1ml2m_unit_test();
    Fr_mul_s1ml2n_unit_test();
    Fr_rawCopyS2L_unit_test();
    Fr_sub_s1s2_unit_test();
    Fr_sub_l1nl2n_unit_test();
    Fr_sub_l1ml2n_unit_test();
    Fr_sub_l1ml2m_unit_test();
    Fr_sub_l1nl2m_unit_test();
    Fr_sub_s1nl2m_unit_test();
    Fr_sub_l1ms2n_unit_test();
    Fr_sub_l1ms2m_unit_test();
    Fr_sub_s1ml2m_unit_test();
    Fr_sub_l1ns2_unit_test();
    Fr_sub_s1l2n_unit_test();
    Fr_add_s1s2_unit_test();
    Fr_add_l1nl2n_unit_test();
    Fr_add_l1ml2n_unit_test();
    Fr_add_l1ml2m_unit_test();
    Fr_add_l1nl2m_unit_test();
    Fr_add_s1nl2m_unit_test();
    Fr_add_l1ms2n_unit_test();
    Fr_add_l1ms2m_unit_test();
    Fr_add_s1ml2m_unit_test();
    Fr_add_l1ns2_unit_test();
    Fr_add_s1l2n_unit_test();
    Fr_geq_s1s2_unit_test();
    Fr_geq_l1nl2n_unit_test();
    Fr_geq_l1ml2n_unit_test();
    Fr_geq_l1ml2m_unit_test();
    Fr_geq_l1nl2m_unit_test();
    Fr_geq_s1l2m_unit_test();
    Fr_geq_l1ms2_unit_test();
    Fr_geq_l1ns2_unit_test();
    Fr_geq_s1l2n_unit_test();
    Fr_eq_s1s2_unit_test();
    Fr_eq_l1nl2n_unit_test();
    Fr_eq_l1ml2n_unit_test();
    Fr_eq_l1ml2m_unit_test();
    Fr_eq_l1nl2m_unit_test();
    Fr_eq_s1l2m_unit_test();
    Fr_eq_l1ms2_unit_test();
    Fr_eq_l1ns2_unit_test();
    Fr_eq_s1l2n_unit_test();
    Fr_neq_s1s2_unit_test();
    Fr_neq_l1nl2n_unit_test();
    Fr_neq_l1ml2n_unit_test();
    Fr_neq_l1ml2m_unit_test();
    Fr_neq_l1nl2m_unit_test();
    Fr_neq_s1l2m_unit_test();
    Fr_neq_l1ms2_unit_test();
    Fr_neq_l1ns2_unit_test();
    Fr_neq_s1l2n_unit_test();
    Fr_gt_s1s2_unit_test();
    Fr_gt_l1nl2n_unit_test();
    Fr_gt_l1ml2n_unit_test();
    Fr_gt_l1ml2m_unit_test();
    Fr_gt_l1nl2m_unit_test();
    Fr_gt_s1l2m_unit_test();
    Fr_gt_l1ms2_unit_test();
    Fr_gt_l1ns2_unit_test();
    Fr_gt_s1l2n_unit_test();
    Fr_band_s1s2_unit_test();
    Fr_band_l1nl2n_unit_test();
    Fr_band_l1ml2n_unit_test();
    Fr_band_l1ml2m_unit_test();
    Fr_band_l1nl2m_unit_test();
    Fr_band_s1l2m_unit_test();
    Fr_band_l1ms2_unit_test();
    Fr_band_l1ns2_unit_test();
    Fr_band_s1l2n_unit_test();
    Fr_land_s1s2_unit_test();
    Fr_land_l1nl2n_unit_test();
    Fr_land_l1ml2n_unit_test();
    Fr_land_l1ml2m_unit_test();
    Fr_land_l1nl2m_unit_test();
    Fr_land_s1l2m_unit_test();
    Fr_land_l1ms2_unit_test();
    Fr_land_l1ns2_unit_test();
    Fr_land_s1l2n_unit_test();
    Fr_lor_s1s2_unit_test();
    Fr_lor_l1nl2n_unit_test();
    Fr_lor_l1ml2n_unit_test();
    Fr_lor_l1ml2m_unit_test();
    Fr_lor_l1nl2m_unit_test();
    Fr_lor_s1l2m_unit_test();
    Fr_lor_l1ms2_unit_test();
    Fr_lor_l1ns2_unit_test();
    Fr_lor_s1l2n_unit_test();
    Fr_lt_s1s2_unit_test();
    Fr_lt_l1nl2n_unit_test();
    Fr_lt_l1ml2n_unit_test();
    Fr_lt_l1ml2m_unit_test();
    Fr_lt_l1nl2m_unit_test();
    Fr_lt_s1l2m_unit_test();
    Fr_lt_l1ms2_unit_test();
    Fr_lt_l1ns2_unit_test();
    Fr_lt_s1l2n_unit_test();
    Fr_toInt_unit_test();
    Fr_neg_unit_test();
    Fr_shr_unit_test();
    Fr_shl_unit_test();
    Fr_rawShr_unit_test();
    Fr_rawShl_unit_test();
    Fr_square_unit_test();
    Fr_bor_unit_test();
    Fr_bxor_unit_test();
    Fr_bnot_unit_test();
    Fr_leq_s1l2n_unit_test();
    Fr_lnot_unit_test();

    Fq_Rw_add_unit_test();
    Fq_Rw_sub_unit_test();
    Fq_Rw_copy_unit_test();
    Fq_Rw_Neg_unit_test();
    Fq_Rw_mul_unit_test();
    Fq_Rw_Msquare_unit_test();
    Fq_Rw_mul1_unit_test();
    Fq_Rw_ToMontgomery_unit_test();
    Fq_Rw_IsEq_unit_test();
    Fq_rawIsZero_unit_test();
    Fq_Rw_FromMontgomery_unit_test();
    Fq_toNormal_unit_test();
    Fq_copy_unit_test();
    Fq_copyn_unit_test();
    Fq_mul_s1s2_unit_test();
    Fq_mul_l1nl2n_unit_test();
    Fq_mul_l1ml2n_unit_test();
    Fq_mul_l1ml2m_unit_test();
    Fq_mul_l1nl2m_unit_test();
    Fq_mul_l1ns2n_unit_test();
    Fq_mul_s1nl2n_unit_test();
    Fq_mul_s1nl2m_unit_test();
    Fq_mul_l1ms2n_unit_test();
    Fq_mul_l1ns2m_unit_test();
    Fq_mul_l1ms2m_unit_test();
    Fq_mul_s1ml2m_unit_test();
    Fq_mul_s1ml2n_unit_test();
    Fq_rawCopyS2L_unit_test();
    Fq_sub_s1s2_unit_test();
    Fq_sub_l1nl2n_unit_test();
    Fq_sub_l1ml2n_unit_test();
    Fq_sub_l1ml2m_unit_test();
    Fq_sub_l1nl2m_unit_test();
    Fq_sub_s1nl2m_unit_test();
    Fq_sub_l1ms2n_unit_test();
    Fq_sub_l1ms2m_unit_test();
    Fq_sub_s1ml2m_unit_test();
    Fq_sub_l1ns2_unit_test();
    Fq_sub_s1l2n_unit_test();
    Fq_add_s1s2_unit_test();
    Fq_add_l1nl2n_unit_test();
    Fq_add_l1ml2n_unit_test();
    Fq_add_l1ml2m_unit_test();
    Fq_add_l1nl2m_unit_test();
    Fq_add_s1nl2m_unit_test();
    Fq_add_l1ms2n_unit_test();
    Fq_add_l1ms2m_unit_test();
    Fq_add_s1ml2m_unit_test();
    Fq_add_l1ns2_unit_test();
    Fq_add_s1l2n_unit_test();
    Fq_geq_s1s2_unit_test();
    Fq_geq_l1nl2n_unit_test();
    Fq_geq_l1ml2n_unit_test();
    Fq_geq_l1ml2m_unit_test();
    Fq_geq_l1nl2m_unit_test();
    Fq_geq_s1l2m_unit_test();
    Fq_geq_l1ms2_unit_test();
    Fq_geq_l1ns2_unit_test();
    Fq_geq_s1l2n_unit_test();
    Fq_eq_s1s2_unit_test();
    Fq_eq_l1nl2n_unit_test();
    Fq_eq_l1ml2n_unit_test();
    Fq_eq_l1ml2m_unit_test();
    Fq_eq_l1nl2m_unit_test();
    Fq_eq_s1l2m_unit_test();
    Fq_eq_l1ms2_unit_test();
    Fq_eq_l1ns2_unit_test();
    Fq_eq_s1l2n_unit_test();
    Fq_neq_s1s2_unit_test();
    Fq_neq_l1nl2n_unit_test();
    Fq_neq_l1ml2n_unit_test();
    Fq_neq_l1ml2m_unit_test();
    Fq_neq_l1nl2m_unit_test();
    Fq_neq_s1l2m_unit_test();
    Fq_neq_l1ms2_unit_test();
    Fq_neq_l1ns2_unit_test();
    Fq_neq_s1l2n_unit_test();
    Fq_gt_s1s2_unit_test();
    Fq_gt_l1nl2n_unit_test();
    Fq_gt_l1ml2n_unit_test();
    Fq_gt_l1ml2m_unit_test();
    Fq_gt_l1nl2m_unit_test();
    Fq_gt_s1l2m_unit_test();
    Fq_gt_l1ms2_unit_test();
    Fq_gt_l1ns2_unit_test();
    Fq_gt_s1l2n_unit_test();
    Fq_band_s1s2_unit_test();
    Fq_band_l1nl2n_unit_test();
    Fq_band_l1ml2n_unit_test();
    Fq_band_l1ml2m_unit_test();
    Fq_band_l1nl2m_unit_test();
    Fq_band_s1l2m_unit_test();
    Fq_band_l1ms2_unit_test();
    Fq_band_l1ns2_unit_test();
    Fq_band_s1l2n_unit_test();
    Fq_land_s1s2_unit_test();
    Fq_land_l1nl2n_unit_test();
    Fq_land_l1ml2n_unit_test();
    Fq_land_l1ml2m_unit_test();
    Fq_land_l1nl2m_unit_test();
    Fq_land_s1l2m_unit_test();
    Fq_land_l1ms2_unit_test();
    Fq_land_l1ns2_unit_test();
    Fq_land_s1l2n_unit_test();
    Fq_lor_s1s2_unit_test();
    Fq_lor_l1nl2n_unit_test();
    Fq_lor_l1ml2n_unit_test();
    Fq_lor_l1ml2m_unit_test();
    Fq_lor_l1nl2m_unit_test();
    Fq_lor_s1l2m_unit_test();
    Fq_lor_l1ms2_unit_test();
    Fq_lor_l1ns2_unit_test();
    Fq_lor_s1l2n_unit_test();
    Fq_lt_s1s2_unit_test();
    Fq_lt_l1nl2n_unit_test();
    Fq_lt_l1ml2n_unit_test();
    Fq_lt_l1ml2m_unit_test();
    Fq_lt_l1nl2m_unit_test();
    Fq_lt_s1l2m_unit_test();
    Fq_lt_l1ms2_unit_test();
    Fq_lt_l1ns2_unit_test();
    Fq_lt_s1l2n_unit_test();
    Fq_toInt_unit_test();
    Fq_neg_unit_test();
    Fq_shr_unit_test();
    Fq_shl_unit_test();
    Fq_rawShr_unit_test();
    Fq_rawShl_unit_test();
    Fq_square_unit_test();
    Fq_bor_unit_test();
    Fq_bxor_unit_test();
    Fq_bnot_unit_test();
    Fq_leq_s1l2n_unit_test();
    Fq_lnot_unit_test();


    print_results();

    return tests_failed ? EXIT_FAILURE : EXIT_SUCCESS;
}
