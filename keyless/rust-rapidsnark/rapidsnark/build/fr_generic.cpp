#include "fr.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

FrElement Fr_q  = {0, 0x80000000, {0x43e1f593f0000001,0x2833e84879b97091,0xb85045b68181585d,0x30644e72e131a029}};
FrElement Fr_R2 = {0, 0x80000000, {0x1bb8e645ae216da7,0x53fe3ab1e35c59e3,0x8c49833d53bb8085,0x0216d0b17f4e44a5}};
FrElement Fr_R3 = {0, 0x80000000, {0x5e94d8e1b4bf0040,0x2a489cbe1cfbb6b8,0x893cc664a19fcfed,0x0cf8594b7fcc657c}};

static FrRawElement half = {0xa1f0fac9f8000000,0x9419f4243cdcb848,0xdc2822db40c0ac2e,0x183227397098d014};


void Fr_copy(PFrElement r, const PFrElement a)
{
    *r = *a;
}

void Fr_toNormal(PFrElement r, PFrElement a)
{
    if (a->type == Fr_LONGMONTGOMERY)
    {
        r->type = Fr_LONG;
        Fr_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        Fr_copy(r, a);
    }
}

static inline int has_mul32_overflow(int64_t val)
{
    int64_t sign = val >> 31;

    if (sign)
    {
        sign = ~sign;
    }

    return sign ? 1 : 0;
}

static inline int Fr_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    int64_t result;

    int overflow = Fr_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fr_rawCopyS2L(r->longVal, result);
        r->type = Fr_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = Fr_SHORT;
        //

        Fr_rawCopyS2L(r->longVal, result);
        r->type = Fr_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
    Fr_rawMMul(r->longVal, r->longVal, Fr_R3.longVal);
}

static inline void mul_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fr_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fr_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fr_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    Fr_rawMMul(r->longVal, r->longVal, Fr_R3.longVal);
}

static inline void mul_s1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fr_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fr_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fr_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    Fr_rawMMul(r->longVal, r->longVal, Fr_R3.longVal);
}

static inline void mul_l1ms2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fr_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fr_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fr_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fr_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fr_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fr_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawMMul(r->longVal, a->longVal, b->longVal);
}

void Fr_mul(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    mul_l1ml2m(r, a, b);
                }
                else
                {
                    mul_l1ml2n(r, a, b);
                }
            }
            else
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            if (b->type & Fr_MONTGOMERY)
            {
                mul_l1ms2m(r, a, b);
            }
            else
            {
                mul_l1ms2n(r, a, b);
            }
        }
        else
        {
            if (b->type & Fr_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (a->type & Fr_MONTGOMERY)
        {
            if (b->type & Fr_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & Fr_MONTGOMERY)
        {
            mul_s1nl2m(r, a, b);
        }
        else
        {
            mul_s1nl2n(r, a, b);
        }
    }
    else
    {
         mul_s1s2(r, a, b);
    }
}

void Fr_toLongNormal(PFrElement r, PFrElement a)
{
    if (a->type & Fr_LONG)
    {
        if (a->type & Fr_MONTGOMERY)
        {
            Fr_rawFromMontgomery(r->longVal, a->longVal);
            r->type = Fr_LONG;
        }
        else
        {
            Fr_copy(r, a);
        }
    }
    else
    {
        Fr_rawCopyS2L(r->longVal, a->shortVal);
        r->type = Fr_LONG;
        r->shortVal = 0;
    }
}

void Fr_toMontgomery(PFrElement r, PFrElement a)
{
    if (a->type & Fr_MONTGOMERY)
    {
        Fr_copy(r, a);
    }
    else if (a->type & Fr_LONG)
    {
        r->shortVal = a->shortVal;

        Fr_rawMMul(r->longVal, a->longVal, Fr_R2.longVal);

        r->type = Fr_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
       int64_t a_shortVal = a->shortVal;
       Fr_rawMMul1(r->longVal, Fr_R2.longVal, -a_shortVal);
       Fr_rawNeg(r->longVal, r->longVal);

       r->type = Fr_SHORTMONTGOMERY;
    }
    else
    {
        Fr_rawMMul1(r->longVal, Fr_R2.longVal, a->shortVal);

        r->type = Fr_SHORTMONTGOMERY;
    }
}

void Fr_copyn(PFrElement r, PFrElement a, int n)
{
    std::memcpy(r, a, n * sizeof(FrElement));
}

static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int Fr_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    int64_t diff;

    int overflow = Fr_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fr_rawCopyS2L(r->longVal, diff);
        r->type = Fr_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fr_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    Fr_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement a_m;
    Fr_toMontgomery(&a_m, a);

    Fr_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement b_m;
    Fr_toMontgomery(&b_m, b);

    Fr_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (a->shortVal >= 0)
    {
        Fr_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fr_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement b_m;
    Fr_toMontgomery(&b_m, b);

    Fr_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement a_m;
    Fr_toMontgomery(&a_m, a);

    Fr_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fr_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        Fr_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PFrElement r,PFrElement a,PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawSub(r->longVal, a->longVal, b->longVal);
}

void Fr_sub(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            if (b->type & Fr_MONTGOMERY)
            {
                sub_l1ms2m(r, a, b);
            }
            else
            {
                sub_l1ms2n(r, a, b);
            }
        }
        else
        {
            sub_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            if (a->type & Fr_MONTGOMERY)
            {
               sub_s1ml2m(r,a,b);
            }
            else
            {
               sub_s1nl2m(r,a,b);
            }
        }
        else
        {
            sub_s1l2n(r,a,b);
        }
    }
    else
    {
         sub_s1s2(r, a, b);
    }
}

static inline int Fr_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    int64_t sum;

    int overflow = Fr_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fr_rawCopyS2L(r->longVal, sum);
        r->type = Fr_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fr_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    Fr_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement a_m;
    Fr_toMontgomery(&a_m, a);

    Fr_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;
    Fr_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement b_m;
    Fr_toMontgomery(&b_m, b);

    Fr_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (a->shortVal >= 0)
    {
        Fr_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fr_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement b_m;

    r->type = Fr_LONGMONTGOMERY;

    Fr_toMontgomery(&b_m, b);

    Fr_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    FrElement m_a;
    Fr_toMontgomery(&m_a, a);

    Fr_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    if (b->shortVal >= 0)
    {
        Fr_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        Fr_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONGMONTGOMERY;

    Fr_rawAdd(r->longVal, a->longVal, b->longVal);
}

void Fr_add(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    add_l1ml2m(r, a, b);
                }
                else
                {
                    add_l1ml2n(r, a, b);
                }
            }
            else
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            if (b->type & Fr_MONTGOMERY)
            {
                add_l1ms2m(r, a, b);
            }
            else
            {
                add_l1ms2n(r, a, b);
            }
        }
        else
        {
            add_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            if (a->type & Fr_MONTGOMERY)
            {
               add_s1ml2m(r, a, b);
            }
            else
            {
               add_s1nl2m(r, a, b);
            }
        }
        else
        {
            add_s1l2n(r, a, b);
        }
    }
    else
    {
        add_s1s2(r, a, b);
    }
}

int Fr_isTrue(PFrElement pE)
{
    int result;

    if (pE->type & Fr_LONG)
    {
        result = !Fr_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int Fr_longNeg(PFrElement pE)
{
    if(Fr_rawCmp(pE->longVal, Fr_q.longVal) >= 0)
    {
       Fr_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - Fr_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       Fr_longErr();
       return 0;
    }

    return result;
}

int Fr_longNormal(PFrElement pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return Fr_longNeg(pE);
    }

    if (pE->longVal[1] || pE->longVal[2] || pE->longVal[3])
    {
        return Fr_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int Fr_toInt(PFrElement pE)
{
    int result;

    if (pE->type & Fr_LONG)
    {
       if (pE->type & Fr_MONTGOMERY)
       {
           FrElement e_n;
           Fr_toNormal(&e_n, pE);

           result = Fr_longNormal(&e_n);
       }
       else
       {
           result = Fr_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PFrElement a, PFrElement b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PFrElement a, PFrElement b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PFrElement a, PFrElement b)
{
    FrElement b_n;

    Fr_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PFrElement a, PFrElement b)
{
    FrElement a_n;

    Fr_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PFrElement a,PFrElement b)
{
    FrElement a_n;

    Fr_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_ln;

    Fr_toLongNormal(&b_ln ,b);
    Fr_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_n;

    Fr_toLongNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PFrElement a, PFrElement b)
{
    FrElement b_n;

    Fr_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t Fr_rlt(PFrElement a, PFrElement b)
{
    int32_t result;

    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            result = rlt_s1l2m(a,b);
        }
        else
        {
            result = rlt_s1l2n(a,b);
        }
    }
    else
    {
         result = rlt_s1s2(a, b);
    }

    return result;
}

void Fr_lt(PFrElement r, PFrElement a, PFrElement b)
{
    r->shortVal = Fr_rlt(a, b);
    r->type = Fr_SHORT;
}

void Fr_geq(PFrElement r, PFrElement a, PFrElement b)
{
   int32_t result = Fr_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fr_SHORT;
}

static inline int Fr_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void Fr_neg(PFrElement r, PFrElement a)
{
    if (a->type & Fr_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        Fr_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = Fr_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            Fr_rawCopyS2L(r->longVal, a_shortVal);
            r->type = Fr_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = Fr_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(FrRawElement pRawA, FrRawElement pRawB)
{
    return Fr_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement a_m;
    Fr_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement b_m;
    Fr_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement a_n;
    Fr_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement b_m;
    Fr_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement a_m;
    Fr_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    FrElement b_n;
    Fr_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int Fr_req(PFrElement r, PFrElement a, PFrElement b)
{
    int result;

    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            result = req_s1l2m(r, a, b);
        }
        else
        {
            result = req_s1l2n(r, a, b);
        }
    }
    else
    {
         result = req_s1s2(r, a, b);
    }

    return result;
}

void Fr_eq(PFrElement r, PFrElement a, PFrElement b)
{
    r->shortVal = Fr_req(r, a, b);
    r->type = Fr_SHORT;
}

void Fr_neq(PFrElement r, PFrElement a, PFrElement b)
{
    int result = Fr_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = Fr_SHORT;
}

// Logical or between two elements
void Fr_lor(PFrElement r, PFrElement a, PFrElement b)
{
    int32_t is_true_a;

    if (a->type & Fr_LONG)
    {
        is_true_a = !Fr_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fr_LONG)
    {
        is_true_b = !Fr_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = Fr_SHORT;
}

void Fr_lnot(PFrElement r, PFrElement a)
{
    if (a->type & Fr_LONG)
    {
        r->shortVal = Fr_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = Fr_SHORT;
}


static inline int rgt_s1s2(PFrElement a, PFrElement b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(FrRawElement pRawA, FrRawElement pRawB)
{
    int result = Fr_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PFrElement a, PFrElement b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PFrElement a, PFrElement b)
{
    FrElement b_n;
    Fr_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PFrElement a, PFrElement b)
{
    FrElement a_n;
    Fr_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PFrElement a, PFrElement b)
{
    FrElement a_n;
    Fr_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PFrElement a, PFrElement b)
{
    FrElement a_n;
    FrElement b_n;

    Fr_toLongNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PFrElement a, PFrElement b)
{
    FrElement b_n;
    Fr_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int Fr_rgt(PFrElement r, PFrElement a, PFrElement b)
{
    int result = 0;

    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            result = rgt_s1l2m(a, b);
        }
        else
        {
            result = rgt_s1l2n(a,b);
        }
    }
    else
    {
         result = rgt_s1s2(a, b);
    }

    return result;
}

void Fr_gt(PFrElement r, PFrElement a, PFrElement b)
{
    r->shortVal = Fr_rgt(r, a, b);
    r->type = Fr_SHORT;
}

void Fr_leq(PFrElement r, PFrElement a, PFrElement b)
{
   int32_t result = Fr_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fr_SHORT;
}

// Logical and between two elements
void Fr_land(PFrElement r, PFrElement a, PFrElement b)
{
    int32_t is_true_a;

    if (a->type & Fr_LONG)
    {
        is_true_a = !Fr_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fr_LONG)
    {
        is_true_b = !Fr_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = Fr_SHORT;
}

static inline void and_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = Fr_SHORT;
        return;
    }

    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toLongNormal(&a_n, a);
    Fr_toLongNormal(&b_n, b);

    Fr_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;
    Fr_toNormal(&b_n, b);

    Fr_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    Fr_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    Fr_toNormal(&a_n, a);

    Fr_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void Fr_band(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            and_s1l2m(r, a, b);
        }
        else
        {
            and_s1l2n(r, a, b);
        }
    }
    else
    {
         and_s1s2(r, a, b);
    }
}

void Fr_rawZero(FrRawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(FrRawElement));
}

static inline void rawShl(FrRawElement r, FrRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fr_rawCopy(r, a);
        return;
    }

    if (b >= 254)
    {
        Fr_rawZero(r);
        return;
    }

    Fr_rawShl(r, a, b);
}

static inline void rawShr(FrRawElement r, FrRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fr_rawCopy(r, a);
        return;
    }

    if (b >= 254)
    {
        Fr_rawZero(r);
        return;
    }

    Fr_rawShr(r,a, b);
}

static inline void Fr_setzero(PFrElement r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PFrElement r, PFrElement a, uint64_t b)
{
    FrElement a_long;
    Fr_toLongNormal(&a_long, a);

    r->type = Fr_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PFrElement r, PFrElement a, uint64_t b)
{
    r->type = Fr_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PFrElement r, PFrElement a, uint64_t b)
{
    if (a->type & Fr_LONG)
    {
        if (a->type == Fr_LONGMONTGOMERY)
        {
            FrElement a_long;
            Fr_toNormal(&a_long, a);

            do_shlln(r, &a_long, b);
        }
        else
        {
            do_shlln(r, a, b);
        }
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            Fr_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            do_shlcl(r, a, b);
        }
        else if(b >= 31)
        {
            do_shlcl(r, a, b);
        }
        else
        {
            a_shortVal <<= b;

            const uint64_t a_is_over_short = a_shortVal >> 31;

            if (a_is_over_short)
            {
                do_shlcl(r, a, b);
            }
            else
            {
                r->type = Fr_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PFrElement r, PFrElement a, uint64_t b)
{
    r->type = Fr_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PFrElement r, PFrElement a, uint64_t b)
{
    if (a->type == Fr_LONGMONTGOMERY)
    {
        FrElement a_long;
        Fr_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PFrElement r, PFrElement a, uint64_t b)
{
    if (a->type & Fr_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            Fr_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            FrElement a_long;
            Fr_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            Fr_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = Fr_SHORT;
        }
    }
}

static inline void Fr_shr_big_shift(PFrElement r, PFrElement a, PFrElement b)
{
    static FrRawElement max_shift = {254, 0, 0, 0};

    FrRawElement shift;

    Fr_rawSubRegular(shift, Fr_q.longVal, b->longVal);

    if (Fr_rawCmp(shift, max_shift) >= 0)
    {
        Fr_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void Fr_shr_long(PFrElement r, PFrElement a, PFrElement b)
{
    static FrRawElement max_shift = {254, 0, 0, 0};

    if (Fr_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fr_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void Fr_shr(PFrElement r, PFrElement a, PFrElement b)
{
    if (b->type & Fr_LONG)
    {
        if (b->type == Fr_LONGMONTGOMERY)
        {
            FrElement b_long;
            Fr_toNormal(&b_long, b);

            Fr_shr_long(r, a, &b_long);
        }
        else
        {
            Fr_shr_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 254)
            {
                Fr_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 254)
        {
            Fr_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void Fr_shl_big_shift(PFrElement r, PFrElement a, PFrElement b)
{
    static FrRawElement max_shift = {254, 0, 0, 0};

    FrRawElement shift;

    Fr_rawSubRegular(shift, Fr_q.longVal, b->longVal);

    if (Fr_rawCmp(shift, max_shift) >= 0)
    {
        Fr_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void Fr_shl_long(PFrElement r, PFrElement a, PFrElement b)
{
    static FrRawElement max_shift = {254, 0, 0, 0};

    if (Fr_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fr_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void Fr_shl(PFrElement r, PFrElement a, PFrElement b)
{
    if (b->type & Fr_LONG)
    {
        if (b->type == Fr_LONGMONTGOMERY)
        {
            FrElement b_long;
            Fr_toNormal(&b_long, b);

            Fr_shl_long(r, a, &b_long);
        }
        else
        {
            Fr_shl_long(r, a, b);
        }
    }
    else
    {
        int64_t b_shortVal = b->shortVal;

        if (b_shortVal < 0)
        {
            b_shortVal = -b_shortVal;

            if (b_shortVal >= 254)
            {
                Fr_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 254)
        {
            Fr_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void Fr_square(PFrElement r, PFrElement a)
{
    if (a->type & Fr_LONG)
    {
        if (a->type == Fr_LONGMONTGOMERY)
        {
            r->type = Fr_LONGMONTGOMERY;
            Fr_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = Fr_LONGMONTGOMERY;
            Fr_rawMSquare(r->longVal, a->longVal);
            Fr_rawMMul(r->longVal, r->longVal, Fr_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = Fr_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            Fr_rawCopyS2L(r->longVal, result);
            r->type = Fr_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = Fr_SHORT;
            //

            Fr_rawCopyS2L(r->longVal, result);
            r->type = Fr_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = Fr_SHORT;
        return;
    }

    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toLongNormal(&a_n, a);
    Fr_toLongNormal(&b_n, b);

    Fr_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;
    Fr_toNormal(&b_n, b);

    Fr_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    Fr_toNormal(&a_n, a);

    Fr_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    Fr_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void Fr_bor(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            or_s1l2m(r, a, b);
        }
        else
        {
            or_s1l2n(r, a, b);
        }
    }
    else
    {
         or_s1s2(r, a, b);
    }
}

static inline void xor_s1s2(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = Fr_SHORT;
        return;
    }

    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toLongNormal(&a_n, a);
    Fr_toLongNormal(&b_n, b);

    Fr_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&a_n, a);
    }

    Fr_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fr_toLongNormal(&b_n, b);
    }

    Fr_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;
    Fr_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement b_n;
    Fr_toNormal(&b_n, b);

    Fr_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    Fr_toNormal(&a_n, a);

    Fr_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PFrElement r, PFrElement a, PFrElement b)
{
    r->type = Fr_LONG;

    FrElement a_n;
    FrElement b_n;

    Fr_toNormal(&a_n, a);
    Fr_toNormal(&b_n, b);

    Fr_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void Fr_bxor(PFrElement r, PFrElement a, PFrElement b)
{
    if (a->type & Fr_LONG)
    {
        if (b->type & Fr_LONG)
        {
            if (a->type & Fr_MONTGOMERY)
            {
                if (b->type & Fr_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fr_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fr_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fr_LONG)
    {
        if (b->type & Fr_MONTGOMERY)
        {
            xor_s1l2m(r, a, b);
        }
        else
        {
            xor_s1l2n(r, a, b);
        }
    }
    else
    {
         xor_s1s2(r, a, b);
    }
}

void Fr_bnot(PFrElement r, PFrElement a)
{
    r->type = Fr_LONG;

    if (a->type == Fr_LONG)
    {
        if (a->type & Fr_MONTGOMERY)
        {
            FrElement a_n;
            Fr_toNormal(&a_n, a);

            Fr_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            Fr_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        FrElement a_n;
        Fr_toLongNormal(&a_n, a);

        Fr_rawNot(r->longVal, a_n.longVal);
    }
}
