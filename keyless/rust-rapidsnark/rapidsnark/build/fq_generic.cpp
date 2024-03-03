#include "fq.hpp"
#include <cstdint>
#include <cstring>
#include <cassert>

FqElement Fq_q  = {0, 0x80000000, {0x3c208c16d87cfd47,0x97816a916871ca8d,0xb85045b68181585d,0x30644e72e131a029}};
FqElement Fq_R2 = {0, 0x80000000, {0xf32cfc5b538afa89,0xb5e71911d44501fb,0x47ab1eff0a417ff6,0x06d89f71cab8351f}};
FqElement Fq_R3 = {0, 0x80000000, {0xb1cd6dafda1530df,0x62f210e6a7283db6,0xef7f0b0c0ada0afb,0x20fd6e902d592544}};

static FqRawElement half = {0x9e10460b6c3e7ea3,0xcbc0b548b438e546,0xdc2822db40c0ac2e,0x183227397098d014};


void Fq_copy(PFqElement r, const PFqElement a)
{
    *r = *a;
}

void Fq_toNormal(PFqElement r, PFqElement a)
{
    if (a->type == Fq_LONGMONTGOMERY)
    {
        r->type = Fq_LONG;
        Fq_rawFromMontgomery(r->longVal, a->longVal);
    }
    else
    {
        Fq_copy(r, a);
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

static inline int Fq_rawSMul(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a * b;

    return has_mul32_overflow(*r);
}

static inline void mul_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    int64_t result;

    int overflow = Fq_rawSMul(&result, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fq_rawCopyS2L(r->longVal, result);
        r->type = Fq_LONG;
        r->shortVal = 0;
    }
    else
    {
        // done the same way as in intel asm implementation
        r->shortVal = (int32_t)result;
        r->type = Fq_SHORT;
        //

        Fq_rawCopyS2L(r->longVal, result);
        r->type = Fq_LONG;
        r->shortVal = 0;
    }
}

static inline void mul_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
    Fq_rawMMul(r->longVal, r->longVal, Fq_R3.longVal);
}

static inline void mul_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ns2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fq_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fq_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fq_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }

    Fq_rawMMul(r->longVal, r->longVal, Fq_R3.longVal);
}

static inline void mul_s1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fq_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fq_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fq_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }

    Fq_rawMMul(r->longVal, r->longVal, Fq_R3.longVal);
}

static inline void mul_l1ms2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fq_rawMMul1(r->longVal, a->longVal, -b_shortVal);
        Fq_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fq_rawMMul1(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void mul_s1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (a->shortVal < 0)
    {
        int64_t a_shortVal = a->shortVal;
        Fq_rawMMul1(r->longVal, b->longVal, -a_shortVal);
        Fq_rawNeg(r->longVal, r->longVal);
    }
    else
    {
        Fq_rawMMul1(r->longVal, b->longVal, a->shortVal);
    }
}

static inline void mul_l1ns2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_l1ms2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

static inline void mul_s1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawMMul(r->longVal, a->longVal, b->longVal);
}

void Fq_mul(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
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
                if (b->type & Fq_MONTGOMERY)
                {
                    mul_l1nl2m(r, a, b);
                }
                else
                {
                    mul_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            if (b->type & Fq_MONTGOMERY)
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
            if (b->type & Fq_MONTGOMERY)
            {
                mul_l1ns2m(r, a, b);
            }
            else
            {
                mul_l1ns2n(r, a, b);
            }
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (a->type & Fq_MONTGOMERY)
        {
            if (b->type & Fq_MONTGOMERY)
            {
                mul_s1ml2m(r, a, b);
            }
            else
            {
                mul_s1ml2n(r,a, b);
            }
        }
        else if (b->type & Fq_MONTGOMERY)
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


void Fq_toLongNormal(PFqElement r, PFqElement a)
{
    if (a->type & Fq_LONG)
    {
        if (a->type & Fq_MONTGOMERY)
        {
            Fq_rawFromMontgomery(r->longVal, a->longVal);
            r->type = Fq_LONG;
        }
        else
        {
            Fq_copy(r, a);
        }
    }
    else
    {
        Fq_rawCopyS2L(r->longVal, a->shortVal);
        r->type = Fq_LONG;
        r->shortVal = 0;
    }
}

void Fq_toMontgomery(PFqElement r, PFqElement a)
{
    if (a->type & Fq_MONTGOMERY)
    {
        Fq_copy(r, a);
    }
    else if (a->type & Fq_LONG)
    {
        r->shortVal = a->shortVal;

        Fq_rawMMul(r->longVal, a->longVal, Fq_R2.longVal);

        r->type = Fq_LONGMONTGOMERY;
    }
    else if (a->shortVal < 0)
    {
       int64_t a_shortVal = a->shortVal;
       Fq_rawMMul1(r->longVal, Fq_R2.longVal, -a_shortVal);
       Fq_rawNeg(r->longVal, r->longVal);

       r->type = Fq_SHORTMONTGOMERY;
    }
    else
    {
        Fq_rawMMul1(r->longVal, Fq_R2.longVal, a->shortVal);

        r->type = Fq_SHORTMONTGOMERY;
    }
}

void Fq_copyn(PFqElement r, PFqElement a, int n)
{
    std::memcpy(r, a, n * sizeof(FqElement));
}


static inline int has_add32_overflow(int64_t val)
{
    int64_t signs = (val >> 31) & 0x3;

    return signs == 1 || signs == 2;
}

static inline int Fq_rawSSub(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a - b;

    return has_add32_overflow(*r);
}

static inline void sub_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    int64_t diff;

    int overflow = Fq_rawSSub(&diff, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fq_rawCopyS2L(r->longVal, diff);
        r->type = Fq_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fq_SHORT;
        r->shortVal = (int32_t)diff;
    }
}

static inline void sub_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    Fq_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement a_m;
    Fq_toMontgomery(&a_m, a);

    Fq_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement b_m;
    Fq_toMontgomery(&b_m, b);

    Fq_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (a->shortVal >= 0)
    {
        Fq_rawSubSL(r->longVal, a->shortVal, b->longVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fq_rawNegLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void sub_l1ms2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement b_m;
    Fq_toMontgomery(&b_m, b);

    Fq_rawSub(r->longVal, a->longVal, b_m.longVal);
}

static inline void sub_s1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement a_m;
    Fq_toMontgomery(&a_m, a);

    Fq_rawSub(r->longVal, a_m.longVal, b->longVal);
}

static inline void sub_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (b->shortVal < 0)
    {
        int64_t b_shortVal = b->shortVal;
        Fq_rawAddLS(r->longVal, a->longVal, -b_shortVal);
    }
    else
    {
        Fq_rawSubLS(r->longVal, a->longVal, b->shortVal);
    }
}

static inline void sub_l1ms2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawSub(r->longVal, a->longVal, b->longVal);
}

static inline void sub_s1ml2m(PFqElement r,PFqElement a,PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawSub(r->longVal, a->longVal, b->longVal);
}

void Fq_sub(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    sub_l1ml2m(r, a, b);
                }
                else
                {
                    sub_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                sub_l1nl2m(r, a, b);
            }
            else
            {
                sub_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            if (b->type & Fq_MONTGOMERY)
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
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
        {
            if (a->type & Fq_MONTGOMERY)
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

static inline int Fq_rawSAdd(int64_t *r, int32_t a, int32_t b)
{
    *r = (int64_t)a + b;

    return has_add32_overflow(*r);
}

static inline void add_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    int64_t sum;

    int overflow = Fq_rawSAdd(&sum, a->shortVal, b->shortVal);

    if (overflow)
    {
        Fq_rawCopyS2L(r->longVal, sum);
        r->type = Fq_LONG;
        r->shortVal = 0;
    }
    else
    {
        r->type = Fq_SHORT;
        r->shortVal = (int32_t)sum;
    }
}

static inline void add_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    Fq_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement a_m;
    Fq_toMontgomery(&a_m, a);

    Fq_rawAdd(r->longVal, a_m.longVal, b->longVal);
}

static inline void add_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;
    Fq_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement b_m;
    Fq_toMontgomery(&b_m, b);

    Fq_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (a->shortVal >= 0)
    {
        Fq_rawAddLS(r->longVal, b->longVal, a->shortVal);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;
        Fq_rawSubLS(r->longVal, b->longVal, -a_shortVal);
    }
}

static inline void add_l1ms2n(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement b_m;

    r->type = Fq_LONGMONTGOMERY;

    Fq_toMontgomery(&b_m, b);

    Fq_rawAdd(r->longVal, a->longVal, b_m.longVal);
}

static inline void add_s1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    FqElement m_a;
    Fq_toMontgomery(&m_a, a);

    Fq_rawAdd(r->longVal, m_a.longVal, b->longVal);
}

static inline void add_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    if (b->shortVal >= 0)
    {
        Fq_rawAddLS(r->longVal, a->longVal, b->shortVal);
    }
    else
    {
        int64_t b_shortVal = b->shortVal;
        Fq_rawSubLS(r->longVal, a->longVal, -b_shortVal);
    }
}

static inline void add_l1ms2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawAdd(r->longVal, a->longVal, b->longVal);
}

static inline void add_s1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONGMONTGOMERY;

    Fq_rawAdd(r->longVal, a->longVal, b->longVal);
}

void Fq_add(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
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
                if (b->type & Fq_MONTGOMERY)
                {
                    add_l1nl2m(r, a, b);
                }
                else
                {
                    add_l1nl2n(r, a, b);
                }
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            if (b->type & Fq_MONTGOMERY)
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
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
        {
            if (a->type & Fq_MONTGOMERY)
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

int Fq_isTrue(PFqElement pE)
{
    int result;

    if (pE->type & Fq_LONG)
    {
        result = !Fq_rawIsZero(pE->longVal);
    }
    else
    {
        result = pE->shortVal != 0;
    }

    return result;
}

int Fq_longNeg(PFqElement pE)
{
    if(Fq_rawCmp(pE->longVal, Fq_q.longVal) >= 0)
    {
       Fq_longErr();
       return 0;
    }

    int64_t result = pE->longVal[0] - Fq_q.longVal[0];

    int64_t is_long = (result >> 31) + 1;

    if(is_long)
    {
       Fq_longErr();
       return 0;
    }

    return result;
}

int Fq_longNormal(PFqElement pE)
{
    uint64_t is_long = 0;
    uint64_t result;

    result = pE->longVal[0];

    is_long = result >> 31;

    if (is_long)
    {
         return Fq_longNeg(pE);
    }

    if (pE->longVal[1] || pE->longVal[2] || pE->longVal[3])
    {
        return Fq_longNeg(pE);
    }

    return result;
}

// Convert a 64 bit integer to a long format field element
int Fq_toInt(PFqElement pE)
{
    int result;

    if (pE->type & Fq_LONG)
    {
       if (pE->type & Fq_MONTGOMERY)
       {
           FqElement e_n;
           Fq_toNormal(&e_n, pE);

           result = Fq_longNormal(&e_n);
       }
       else
       {
           result = Fq_longNormal(pE);
       }
    }
    else
    {
        result = pE->shortVal;
    }

    return result;
}

static inline int rlt_s1s2(PFqElement a, PFqElement b)
{
    return (a->shortVal < b->shortVal) ? 1 : 0;
}

static inline int rltRawL1L2(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(pRawB, pRawA);

    return result > 0 ? 1 : 0;
}

static inline int rltl1l2_n1(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rltRawL1L2(pRawA, pRawB);
    }

     return 1;
}

static inline int rltl1l2_p1(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 0;
    }

    return rltRawL1L2(pRawA, pRawB);
}

static inline int rltL1L2(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rltl1l2_n1(pRawA, pRawB);
    }

    return rltl1l2_p1(pRawA, pRawB);
}

static inline int rlt_l1nl2n(PFqElement a, PFqElement b)
{
    return rltL1L2(a->longVal, b->longVal);
}

static inline int rlt_l1nl2m(PFqElement a, PFqElement b)
{
    FqElement b_n;

    Fq_toNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

static inline int rlt_l1ml2m(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ml2n(PFqElement a, PFqElement b)
{
    FqElement a_n;

    Fq_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_s1l2n(PFqElement a,PFqElement b)
{
    FqElement a_n;

    Fq_toLongNormal(&a_n,a);

    return rltL1L2(a_n.longVal, b->longVal);
}

static inline int rlt_l1ms2(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_ln;

    Fq_toLongNormal(&b_ln ,b);
    Fq_toNormal(&a_n, a);

    return rltL1L2(a_n.longVal, b_ln.longVal);
}

static inline int rlt_s1l2m(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_n;

    Fq_toLongNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    return rltL1L2(a_n.longVal, b_n.longVal);
}

static inline int rlt_l1ns2(PFqElement a, PFqElement b)
{
    FqElement b_n;

    Fq_toLongNormal(&b_n, b);

    return rltL1L2(a->longVal, b_n.longVal);
}

int32_t Fq_rlt(PFqElement a, PFqElement b)
{
    int32_t result;

    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    result = rlt_l1ml2m(a, b);
                }
                else
                {
                    result = rlt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                result = rlt_l1nl2m(a, b);
            }
            else
            {
                result = rlt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            result = rlt_l1ms2(a, b);
        }
        else
        {
            result = rlt_l1ns2(a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

void Fq_lt(PFqElement r, PFqElement a, PFqElement b)
{
    r->shortVal = Fq_rlt(a, b);
    r->type = Fq_SHORT;
}

void Fq_geq(PFqElement r, PFqElement a, PFqElement b)
{
   int32_t result = Fq_rlt(a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fq_SHORT;
}

static inline int Fq_rawSNeg(int64_t *r, int32_t a)
{
    *r = -(int64_t)a;

    return has_add32_overflow(*r);
}

void Fq_neg(PFqElement r, PFqElement a)
{
    if (a->type & Fq_LONG)
    {
        r->type = a->type;
        r->shortVal = a->shortVal;
        Fq_rawNeg(r->longVal, a->longVal);
    }
    else
    {
        int64_t a_shortVal;

        int overflow = Fq_rawSNeg(&a_shortVal, a->shortVal);

        if (overflow)
        {
            Fq_rawCopyS2L(r->longVal, a_shortVal);
            r->type = Fq_LONG;
            r->shortVal = 0;
        }
        else
        {
            r->type = Fq_SHORT;
            r->shortVal = (int32_t)a_shortVal;
        }
    }
}

static inline int reqL1L2(FqRawElement pRawA, FqRawElement pRawB)
{
    return Fq_rawCmp(pRawB, pRawA) == 0;
}

static inline int req_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    return (a->shortVal == b->shortVal) ? 1 : 0;
}

static inline int req_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement a_m;
    Fq_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    return reqL1L2(a->longVal, b->longVal);
}

static inline int req_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement b_m;
    Fq_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement a_n;
    Fq_toLongNormal(&a_n, a);

    return reqL1L2(a_n.longVal, b->longVal);
}

static inline int req_l1ms2(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement b_m;
    Fq_toMontgomery(&b_m, b);

    return reqL1L2(a->longVal, b_m.longVal);
}

static inline int req_s1l2m(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement a_m;
    Fq_toMontgomery(&a_m, a);

    return reqL1L2(a_m.longVal, b->longVal);
}

static inline int req_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    FqElement b_n;
    Fq_toLongNormal(&b_n, b);

    return reqL1L2(a->longVal, b_n.longVal);
}

// Compares two elements of any kind
int Fq_req(PFqElement r, PFqElement a, PFqElement b)
{
    int result;

    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    result = req_l1ml2m(r, a, b);
                }
                else
                {
                    result = req_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                result = req_l1nl2m(r, a, b);
            }
            else
            {
                result = req_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            result = req_l1ms2(r, a, b);
        }
        else
        {
            result = req_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

void Fq_eq(PFqElement r, PFqElement a, PFqElement b)
{
    r->shortVal = Fq_req(r, a, b);
    r->type = Fq_SHORT;
}

void Fq_neq(PFqElement r, PFqElement a, PFqElement b)
{
    int result = Fq_req(r, a, b);

    r->shortVal = result ^ 0x1;
    r->type = Fq_SHORT;
}

// Logical or between two elements
void Fq_lor(PFqElement r, PFqElement a, PFqElement b)
{
    int32_t is_true_a;

    if (a->type & Fq_LONG)
    {
        is_true_a = !Fq_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fq_LONG)
    {
        is_true_b = !Fq_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a | is_true_b;
    r->type = Fq_SHORT;
}

void Fq_lnot(PFqElement r, PFqElement a)
{
    if (a->type & Fq_LONG)
    {
        r->shortVal = Fq_rawIsZero(a->longVal);
    }
    else
    {
        r->shortVal = a->shortVal ? 0 : 1;
    }

    r->type = Fq_SHORT;
}


static inline int rgt_s1s2(PFqElement a, PFqElement b)
{
    return (a->shortVal > b->shortVal) ? 1 : 0;
}

static inline int rgtRawL1L2(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(pRawB, pRawA);

    return (result < 0) ? 1 : 0;
}

static inline int rgtl1l2_n1(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawB);

    if (result < 0)
    {
        return rgtRawL1L2(pRawA, pRawB);
    }
    return 0;
}

static inline int rgtl1l2_p1(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawB);

    if (result < 0)
    {
        return 1;
    }
    return rgtRawL1L2(pRawA, pRawB);
}

static inline int rgtL1L2(FqRawElement pRawA, FqRawElement pRawB)
{
    int result = Fq_rawCmp(half, pRawA);

    if (result < 0)
    {
        return rgtl1l2_n1(pRawA, pRawB);
    }

    return rgtl1l2_p1(pRawA, pRawB);
}

static inline int rgt_l1nl2n(PFqElement a, PFqElement b)
{
    return rgtL1L2(a->longVal, b->longVal);
}

static inline int rgt_l1nl2m(PFqElement a, PFqElement b)
{
    FqElement b_n;
    Fq_toNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

static inline int rgt_l1ml2m(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ml2n(PFqElement a, PFqElement b)
{
    FqElement a_n;
    Fq_toNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_s1l2n(PFqElement a, PFqElement b)
{
    FqElement a_n;
    Fq_toLongNormal(&a_n, a);

    return rgtL1L2(a_n.longVal, b->longVal);
}

static inline int rgt_l1ms2(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toLongNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_s1l2m(PFqElement a, PFqElement b)
{
    FqElement a_n;
    FqElement b_n;

    Fq_toLongNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    return rgtL1L2(a_n.longVal, b_n.longVal);
}

static inline int rgt_l1ns2(PFqElement a, PFqElement b)
{
    FqElement b_n;
    Fq_toLongNormal(&b_n, b);

    return rgtL1L2(a->longVal, b_n.longVal);
}

int Fq_rgt(PFqElement r, PFqElement a, PFqElement b)
{
    int result = 0;

    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    result = rgt_l1ml2m(a, b);
                }
                else
                {
                    result = rgt_l1ml2n(a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                result = rgt_l1nl2m(a, b);
            }
            else
            {
                result = rgt_l1nl2n(a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            result = rgt_l1ms2(a, b);
        }
        else
        {
            result = rgt_l1ns2(a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

void Fq_gt(PFqElement r, PFqElement a, PFqElement b)
{
    r->shortVal = Fq_rgt(r, a, b);
    r->type = Fq_SHORT;
}

void Fq_leq(PFqElement r, PFqElement a, PFqElement b)
{
   int32_t result = Fq_rgt(r, a, b);
   result ^= 0x1;

   r->shortVal = result;
   r->type = Fq_SHORT;
}

// Logical and between two elements
void Fq_land(PFqElement r, PFqElement a, PFqElement b)
{
    int32_t is_true_a;

    if (a->type & Fq_LONG)
    {
        is_true_a = !Fq_rawIsZero(a->longVal);
    }
    else
    {
        is_true_a = a->shortVal ? 1 : 0;
    }

    int32_t is_true_b;

    if (b->type & Fq_LONG)
    {
        is_true_b = !Fq_rawIsZero(b->longVal);
    }
    else
    {
        is_true_b = b->shortVal ? 1 : 0;
    }

    r->shortVal = is_true_a & is_true_b;
    r->type = Fq_SHORT;
}

static inline void and_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        int32_t result = a->shortVal & b->shortVal;
        r->shortVal = result;
        r->type = Fq_SHORT;
        return;
    }

    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toLongNormal(&a_n, a);
    Fq_toLongNormal(&b_n, b);

    Fq_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawAnd(r->longVal, a->longVal, b->longVal);
}

static inline void and_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;
    Fq_toNormal(&b_n, b);

    Fq_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

static inline void and_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    Fq_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    Fq_toNormal(&a_n, a);

    Fq_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawAnd(r->longVal, a_n.longVal, b->longVal);
}

static inline void and_l1ms2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawAnd(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void and_s1l2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawAnd(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void and_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawAnd(r->longVal, a->longVal, b_n.longVal);
}

// Ands two elements of any kind
void Fq_band(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    and_l1ml2m(r, a, b);
                }
                else
                {
                    and_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                and_l1nl2m(r, a, b);
            }
            else
            {
                and_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            and_l1ms2(r, a, b);
        }
        else
        {
           and_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

void Fq_rawZero(FqRawElement pRawResult)
{
    std::memset(pRawResult, 0, sizeof(FqRawElement));
}

static inline void rawShl(FqRawElement r, FqRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fq_rawCopy(r, a);
        return;
    }

    if (b >= 254)
    {
        Fq_rawZero(r);
        return;
    }

    Fq_rawShl(r, a, b);
}

static inline void rawShr(FqRawElement r, FqRawElement a, uint64_t b)
{
    if (b == 0)
    {
        Fq_rawCopy(r, a);
        return;
    }

    if (b >= 254)
    {
        Fq_rawZero(r);
        return;
    }

    Fq_rawShr(r,a, b);
}

static inline void Fq_setzero(PFqElement r)
{
    r->type = 0;
    r->shortVal = 0;
}

static inline void do_shlcl(PFqElement r, PFqElement a, uint64_t b)
{
    FqElement a_long;
    Fq_toLongNormal(&a_long, a);

    r->type = Fq_LONG;
    rawShl(r->longVal, a_long.longVal, b);
}

static inline void do_shlln(PFqElement r, PFqElement a, uint64_t b)
{
    r->type = Fq_LONG;
    rawShl(r->longVal, a->longVal, b);
}

static inline void do_shl(PFqElement r, PFqElement a, uint64_t b)
{
    if (a->type & Fq_LONG)
    {
        if (a->type == Fq_LONGMONTGOMERY)
        {
            FqElement a_long;
            Fq_toNormal(&a_long, a);

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
            Fq_setzero(r);
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
                r->type = Fq_SHORT;
                r->shortVal = a_shortVal;
            }
        }
    }
}

static inline void do_shrln(PFqElement r, PFqElement a, uint64_t b)
{
    r->type = Fq_LONG;
    rawShr(r->longVal, a->longVal, b);
}

static inline void do_shrl(PFqElement r, PFqElement a, uint64_t b)
{
    if (a->type == Fq_LONGMONTGOMERY)
    {
        FqElement a_long;
        Fq_toNormal(&a_long, a);

        do_shrln(r, &a_long, b);
    }
    else
    {
        do_shrln(r, a, b);
    }
}

static inline void do_shr(PFqElement r, PFqElement a, uint64_t b)
{
    if (a->type & Fq_LONG)
    {
        do_shrl(r, a, b);
    }
    else
    {
        int64_t a_shortVal = a->shortVal;

        if (a_shortVal == 0)
        {
            Fq_setzero(r);
        }
        else if (a_shortVal < 0)
        {
            FqElement a_long;
            Fq_toLongNormal(&a_long, a);

            do_shrl(r, &a_long, b);
        }
        else if(b >= 31)
        {
            Fq_setzero(r);
        }
        else
        {
            a_shortVal >>= b;

            r->shortVal = a_shortVal;
            r->type = Fq_SHORT;
        }
    }
}

static inline void Fq_shr_big_shift(PFqElement r, PFqElement a, PFqElement b)
{
    static FqRawElement max_shift = {254, 0, 0, 0};

    FqRawElement shift;

    Fq_rawSubRegular(shift, Fq_q.longVal, b->longVal);

    if (Fq_rawCmp(shift, max_shift) >= 0)
    {
        Fq_setzero(r);
    }
    else
    {
        do_shl(r, a, shift[0]);
    }
}

static inline void Fq_shr_long(PFqElement r, PFqElement a, PFqElement b)
{
    static FqRawElement max_shift = {254, 0, 0, 0};

    if (Fq_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fq_shr_big_shift(r, a, b);
    }
    else
    {
        do_shr(r, a, b->longVal[0]);
    }
}

void Fq_shr(PFqElement r, PFqElement a, PFqElement b)
{
    if (b->type & Fq_LONG)
    {
        if (b->type == Fq_LONGMONTGOMERY)
        {
            FqElement b_long;
            Fq_toNormal(&b_long, b);

            Fq_shr_long(r, a, &b_long);
        }
        else
        {
            Fq_shr_long(r, a, b);
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
                Fq_setzero(r);
            }
            else
            {
                do_shl(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 254)
        {
            Fq_setzero(r);
        }
        else
        {
            do_shr(r, a, b_shortVal);
        }
    }
}

static inline void Fq_shl_big_shift(PFqElement r, PFqElement a, PFqElement b)
{
    static FqRawElement max_shift = {254, 0, 0, 0};

    FqRawElement shift;

    Fq_rawSubRegular(shift, Fq_q.longVal, b->longVal);

    if (Fq_rawCmp(shift, max_shift) >= 0)
    {
        Fq_setzero(r);
    }
    else
    {
        do_shr(r, a, shift[0]);
    }
}

static inline void Fq_shl_long(PFqElement r, PFqElement a, PFqElement b)
{
    static FqRawElement max_shift = {254, 0, 0, 0};

    if (Fq_rawCmp(b->longVal, max_shift) >= 0)
    {
        Fq_shl_big_shift(r, a, b);
    }
    else
    {
        do_shl(r, a, b->longVal[0]);
    }
}

void Fq_shl(PFqElement r, PFqElement a, PFqElement b)
{
    if (b->type & Fq_LONG)
    {
        if (b->type == Fq_LONGMONTGOMERY)
        {
            FqElement b_long;
            Fq_toNormal(&b_long, b);

            Fq_shl_long(r, a, &b_long);
        }
        else
        {
            Fq_shl_long(r, a, b);
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
                Fq_setzero(r);
            }
            else
            {
                do_shr(r, a, b_shortVal);
            }
        }
        else if (b_shortVal >= 254)
        {
            Fq_setzero(r);
        }
        else
        {
            do_shl(r, a, b_shortVal);
        }
    }
}

void Fq_square(PFqElement r, PFqElement a)
{
    if (a->type & Fq_LONG)
    {
        if (a->type == Fq_LONGMONTGOMERY)
        {
            r->type = Fq_LONGMONTGOMERY;
            Fq_rawMSquare(r->longVal, a->longVal);
        }
        else
        {
            r->type = Fq_LONGMONTGOMERY;
            Fq_rawMSquare(r->longVal, a->longVal);
            Fq_rawMMul(r->longVal, r->longVal, Fq_R3.longVal);
        }
    }
    else
    {
        int64_t result;

        int overflow = Fq_rawSMul(&result, a->shortVal, a->shortVal);

        if (overflow)
        {
            Fq_rawCopyS2L(r->longVal, result);
            r->type = Fq_LONG;
            r->shortVal = 0;
        }
        else
        {
            // done the same way as in intel asm implementation
            r->shortVal = (int32_t)result;
            r->type = Fq_SHORT;
            //

            Fq_rawCopyS2L(r->longVal, result);
            r->type = Fq_LONG;
            r->shortVal = 0;
        }
    }
}

static inline void or_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal | b->shortVal;
        r->type = Fq_SHORT;
        return;
    }

    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toLongNormal(&a_n, a);
    Fq_toLongNormal(&b_n, b);

    Fq_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void or_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ms2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawOr(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void or_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawOr(r->longVal, a->longVal, b->longVal);
}

static inline void or_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;
    Fq_toNormal(&b_n, b);

    Fq_rawOr(r->longVal, a->longVal, b_n.longVal);
}

static inline void or_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    Fq_toNormal(&a_n, a);

    Fq_rawOr(r->longVal, a_n.longVal, b->longVal);
}

static inline void or_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    Fq_rawOr(r->longVal, a_n.longVal, b_n.longVal);
}


void Fq_bor(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    or_l1ml2m(r, a, b);
                }
                else
                {
                    or_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                or_l1nl2m(r, a, b);
            }
            else
            {
                or_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            or_l1ms2(r, a, b);
        }
        else
        {
           or_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

static inline void xor_s1s2(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->shortVal >= 0 && b->shortVal >= 0)
    {
        r->shortVal = a->shortVal ^ b->shortVal;
        r->type = Fq_SHORT;
        return;
    }

    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toLongNormal(&a_n, a);
    Fq_toLongNormal(&b_n, b);

    Fq_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_s1l2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_s1l2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&b_n, b);

    if (a->shortVal >= 0)
    {
        a_n = {0, 0, {(uint64_t)a->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&a_n, a);
    }

    Fq_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

static inline void xor_l1ns2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ms2(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);

    if (b->shortVal >= 0)
    {
        b_n = {0, 0, {(uint64_t)b->shortVal, 0, 0, 0}};
    }
    else
    {
        Fq_toLongNormal(&b_n, b);
    }

    Fq_rawXor(r->longVal, b_n.longVal, a_n.longVal);
}

static inline void xor_l1nl2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;
    Fq_rawXor(r->longVal, a->longVal, b->longVal);
}

static inline void xor_l1nl2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement b_n;
    Fq_toNormal(&b_n, b);

    Fq_rawXor(r->longVal, a->longVal, b_n.longVal);
}

static inline void xor_l1ml2n(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    Fq_toNormal(&a_n, a);

    Fq_rawXor(r->longVal, a_n.longVal, b->longVal);
}

static inline void xor_l1ml2m(PFqElement r, PFqElement a, PFqElement b)
{
    r->type = Fq_LONG;

    FqElement a_n;
    FqElement b_n;

    Fq_toNormal(&a_n, a);
    Fq_toNormal(&b_n, b);

    Fq_rawXor(r->longVal, a_n.longVal, b_n.longVal);
}

void Fq_bxor(PFqElement r, PFqElement a, PFqElement b)
{
    if (a->type & Fq_LONG)
    {
        if (b->type & Fq_LONG)
        {
            if (a->type & Fq_MONTGOMERY)
            {
                if (b->type & Fq_MONTGOMERY)
                {
                    xor_l1ml2m(r, a, b);
                }
                else
                {
                    xor_l1ml2n(r, a, b);
                }
            }
            else if (b->type & Fq_MONTGOMERY)
            {
                xor_l1nl2m(r, a, b);
            }
            else
            {
                xor_l1nl2n(r, a, b);
            }
        }
        else if (a->type & Fq_MONTGOMERY)
        {
            xor_l1ms2(r, a, b);
        }
        else
        {
           xor_l1ns2(r, a, b);
        }
    }
    else if (b->type & Fq_LONG)
    {
        if (b->type & Fq_MONTGOMERY)
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

void Fq_bnot(PFqElement r, PFqElement a)
{
    r->type = Fq_LONG;

    if (a->type == Fq_LONG)
    {
        if (a->type & Fq_MONTGOMERY)
        {
            FqElement a_n;
            Fq_toNormal(&a_n, a);

            Fq_rawNot(r->longVal, a_n.longVal);
        }
        else
        {
            Fq_rawNot(r->longVal, a->longVal);
        }
    }
    else
    {
        FqElement a_n;
        Fq_toLongNormal(&a_n, a);

        Fq_rawNot(r->longVal, a_n.longVal);
    }
}
