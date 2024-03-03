#ifndef __FR_H
#define __FR_H

#include "fr_element.hpp"
#include <cstdint>
#include <string>
#include <gmp.h>

#ifdef __APPLE__
#include <sys/types.h> // typedef unsigned int uint;
#endif // __APPLE__

extern FrElement Fr_q;
extern FrElement Fr_R2;
extern FrElement Fr_R3;
extern FrRawElement Fr_rawq;
extern FrRawElement Fr_rawR3;

#ifdef USE_ASM

#if defined(ARCH_X86_64)

extern "C" void Fr_copy(PFrElement r, PFrElement a);
extern "C" void Fr_copyn(PFrElement r, PFrElement a, int n);
extern "C" void Fr_add(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_sub(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_neg(PFrElement r, PFrElement a);
extern "C" void Fr_mul(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_square(PFrElement r, PFrElement a);
extern "C" void Fr_band(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_bor(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_bxor(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_bnot(PFrElement r, PFrElement a);
extern "C" void Fr_shl(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_shr(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_eq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_neq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lt(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_gt(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_leq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_geq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_land(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lor(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lnot(PFrElement r, PFrElement a);
extern "C" void Fr_toNormal(PFrElement r, PFrElement a);
extern "C" void Fr_toLongNormal(PFrElement r, PFrElement a);
extern "C" void Fr_toMontgomery(PFrElement r, PFrElement a);

extern "C" int Fr_isTrue(PFrElement pE);
extern "C" int Fr_toInt(PFrElement pE);

extern "C" void Fr_rawCopy(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawSwap(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawAdd(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" void Fr_rawSub(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" void Fr_rawNeg(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawMMul(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" void Fr_rawMSquare(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawMMul1(FrRawElement pRawResult, const FrRawElement pRawA, uint64_t pRawB);
extern "C" void Fr_rawToMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
extern "C" void Fr_rawFromMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
extern "C" int Fr_rawIsEq(const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" int Fr_rawIsZero(const FrRawElement pRawB);
extern "C" void Fr_rawShl(FrRawElement r, FrRawElement a, uint64_t b);
extern "C" void Fr_rawShr(FrRawElement r, FrRawElement a, uint64_t b);

extern "C" void Fr_fail();

#elif defined(ARCH_ARM64)

           void Fr_copy(PFrElement r, PFrElement a);
           void Fr_mul(PFrElement r, PFrElement a, PFrElement b);
           void Fr_toNormal(PFrElement r, PFrElement a);

           void Fr_toLongNormal(PFrElement r, PFrElement a);
           int  Fr_isTrue(PFrElement pE);
           void Fr_copyn(PFrElement r, PFrElement a, int n);
           void Fr_lt(PFrElement r, PFrElement a, PFrElement b);
           int  Fr_toInt(PFrElement pE);
           void Fr_shr(PFrElement r, PFrElement a, PFrElement b);
           void Fr_shl(PFrElement r, PFrElement a, PFrElement b);
           void Fr_band(PFrElement r, PFrElement a, PFrElement b);
           void Fr_bor(PFrElement r, PFrElement a, PFrElement b);
           void Fr_bxor(PFrElement r, PFrElement a, PFrElement b);
           void Fr_bnot(PFrElement r, PFrElement a);
           void Fr_sub(PFrElement r, PFrElement a, PFrElement b);
           void Fr_eq(PFrElement r, PFrElement a, PFrElement b);
           void Fr_neq(PFrElement r, PFrElement a, PFrElement b);
           void Fr_add(PFrElement r, PFrElement a, PFrElement b);
           void Fr_gt(PFrElement r, PFrElement a, PFrElement b);
           void Fr_leq(PFrElement r, PFrElement a, PFrElement b);
           void Fr_geq(PFrElement r, PFrElement a, PFrElement b);
           void Fr_lor(PFrElement r, PFrElement a, PFrElement b);
           void Fr_lnot(PFrElement r, PFrElement a);
           void Fr_land(PFrElement r, PFrElement a, PFrElement b);
           void Fr_neg(PFrElement r, PFrElement a);
           void Fr_toMontgomery(PFrElement r, PFrElement a);
           void Fr_square(PFrElement r, PFrElement a);

extern "C" void Fr_rawCopy(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawSwap(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawAdd(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" void Fr_rawSub(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" void Fr_rawNeg(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawMMul(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
           void Fr_rawMSquare(FrRawElement pRawResult, const FrRawElement pRawA);
extern "C" void Fr_rawMMul1(FrRawElement pRawResult, const FrRawElement pRawA, uint64_t pRawB);
           void Fr_rawToMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
extern "C" void Fr_rawFromMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
extern "C" int  Fr_rawIsEq(const FrRawElement pRawA, const FrRawElement pRawB);
extern "C" int  Fr_rawIsZero(const FrRawElement pRawB);
           void Fr_rawZero(FrRawElement pRawResult);
extern "C" void Fr_rawCopyS2L(FrRawElement pRawResult, int64_t val);
extern "C" void Fr_rawAddLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
extern "C" void Fr_rawSubSL(FrRawElement pRawResult, uint64_t rawA, FrRawElement pRawB);
extern "C" void Fr_rawSubLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
extern "C" void Fr_rawNegLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
extern "C" int  Fr_rawCmp(FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawAnd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawOr(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawXor(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawShl(FrRawElement r, FrRawElement a, uint64_t b);
extern "C" void Fr_rawShr(FrRawElement r, FrRawElement a, uint64_t b);
extern "C" void Fr_rawNot(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawSubRegular(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);

           void Fr_fail();
           void Fr_longErr();

#endif

#else


void Fr_copy(PFrElement r, PFrElement a);
void Fr_mul(PFrElement r, PFrElement a, PFrElement b);
void Fr_toNormal(PFrElement r, PFrElement a);

void Fr_toLongNormal(PFrElement r, PFrElement a);
int Fr_isTrue(PFrElement pE);
void Fr_copyn(PFrElement r, PFrElement a, int n);
void Fr_lt(PFrElement r, PFrElement a, PFrElement b);
int Fr_toInt(PFrElement pE);
void Fr_shl(PFrElement r, PFrElement a, PFrElement b);
void Fr_shr(PFrElement r, PFrElement a, PFrElement b);
void Fr_band(PFrElement r, PFrElement a, PFrElement b);
void Fr_bor(PFrElement r, PFrElement a, PFrElement b);
void Fr_bxor(PFrElement r, PFrElement a, PFrElement b);
void Fr_bnot(PFrElement r, PFrElement a);
void Fr_sub(PFrElement r, PFrElement a, PFrElement b);
void Fr_eq(PFrElement r, PFrElement a, PFrElement b);
void Fr_neq(PFrElement r, PFrElement a, PFrElement b);
void Fr_add(PFrElement r, PFrElement a, PFrElement b);
void Fr_gt(PFrElement r, PFrElement a, PFrElement b);
void Fr_leq(PFrElement r, PFrElement a, PFrElement b);
void Fr_geq(PFrElement r, PFrElement a, PFrElement b);
void Fr_lor(PFrElement r, PFrElement a, PFrElement b);
void Fr_lnot(PFrElement r, PFrElement a);
void Fr_land(PFrElement r, PFrElement a, PFrElement b);
void Fr_neg(PFrElement r, PFrElement a);
void Fr_toMontgomery(PFrElement r, PFrElement a);
void Fr_square(PFrElement r, PFrElement a);

void Fr_rawCopy(FrRawElement pRawResult, const FrRawElement pRawA);
void Fr_rawSwap(FrRawElement pRawResult, FrRawElement pRawA);
void Fr_rawAdd(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
void Fr_rawSub(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
void Fr_rawNeg(FrRawElement pRawResult, const FrRawElement pRawA);
void Fr_rawMMul(FrRawElement pRawResult, const FrRawElement pRawA, const FrRawElement pRawB);
void Fr_rawMSquare(FrRawElement pRawResult, const FrRawElement pRawA);
void Fr_rawMMul1(FrRawElement pRawResult, const FrRawElement pRawA, uint64_t pRawB);
void Fr_rawToMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
void Fr_rawFromMontgomery(FrRawElement pRawResult, const FrRawElement &pRawA);
int Fr_rawIsEq(const FrRawElement pRawA, const FrRawElement pRawB);
int Fr_rawIsZero(const FrRawElement pRawB);
void Fr_rawZero(FrRawElement pRawResult);
void Fr_rawCopyS2L(FrRawElement pRawResult, int64_t val);
void Fr_rawAddLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
void Fr_rawSubSL(FrRawElement pRawResult, uint64_t rawA, FrRawElement pRawB);
void Fr_rawSubLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
void Fr_rawNegLS(FrRawElement pRawResult, FrRawElement pRawA, uint64_t rawB);
int  Fr_rawCmp(FrRawElement pRawA, FrRawElement pRawB);
void Fr_rawAnd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
void Fr_rawOr(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
void Fr_rawXor(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
void Fr_rawShl(FrRawElement r, FrRawElement a, uint64_t b);
void Fr_rawShr(FrRawElement r, FrRawElement a, uint64_t b);
void Fr_rawNot(FrRawElement pRawResult, FrRawElement pRawA);
void Fr_rawSubRegular(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);

void Fr_fail();
void Fr_longErr();

#endif

// Pending functions to convert

void Fr_str2element(PFrElement pE, char const*s, uint base);
char *Fr_element2str(PFrElement pE);
void Fr_idiv(PFrElement r, PFrElement a, PFrElement b);
void Fr_mod(PFrElement r, PFrElement a, PFrElement b);
void Fr_inv(PFrElement r, PFrElement a);
void Fr_div(PFrElement r, PFrElement a, PFrElement b);
void Fr_pow(PFrElement r, PFrElement a, PFrElement b);

class RawFr {

public:
    const static int N64 = Fr_N64;
    const static int MaxBits = 254;


    struct Element {
        FrRawElement v;
    };

private:
    Element fZero;
    Element fOne;
    Element fNegOne;

public:

    RawFr();
    ~RawFr();

    const Element &zero() { return fZero; };
    const Element &one() { return fOne; };
    const Element &negOne() { return fNegOne; };
    Element set(int value);
    void set(Element &r, int value);

    void fromString(Element &r, const std::string &n, uint32_t radix = 10);
    std::string toString(const Element &a, uint32_t radix = 10);

    void inline copy(Element &r, const Element &a) { Fr_rawCopy(r.v, a.v); };
    void inline swap(Element &a, Element &b) { Fr_rawSwap(a.v, b.v); };
    void inline add(Element &r, const Element &a, const Element &b) { Fr_rawAdd(r.v, a.v, b.v); };
    void inline sub(Element &r, const Element &a, const Element &b) { Fr_rawSub(r.v, a.v, b.v); };
    void inline mul(Element &r, const Element &a, const Element &b) { Fr_rawMMul(r.v, a.v, b.v); };

    Element inline add(const Element &a, const Element &b) { Element r; Fr_rawAdd(r.v, a.v, b.v); return r;};
    Element inline sub(const Element &a, const Element &b) { Element r; Fr_rawSub(r.v, a.v, b.v); return r;};
    Element inline mul(const Element &a, const Element &b) { Element r; Fr_rawMMul(r.v, a.v, b.v); return r;};

    Element inline neg(const Element &a) { Element r; Fr_rawNeg(r.v, a.v); return r; };
    Element inline square(const Element &a) { Element r; Fr_rawMSquare(r.v, a.v); return r; };

    Element inline add(int a, const Element &b) { return add(set(a), b);};
    Element inline sub(int a, const Element &b) { return sub(set(a), b);};
    Element inline mul(int a, const Element &b) { return mul(set(a), b);};

    Element inline add(const Element &a, int b) { return add(a, set(b));};
    Element inline sub(const Element &a, int b) { return sub(a, set(b));};
    Element inline mul(const Element &a, int b) { return mul(a, set(b));};

    void inline mul1(Element &r, const Element &a, uint64_t b) { Fr_rawMMul1(r.v, a.v, b); };
    void inline neg(Element &r, const Element &a) { Fr_rawNeg(r.v, a.v); };
    void inline square(Element &r, const Element &a) { Fr_rawMSquare(r.v, a.v); };
    void inv(Element &r, const Element &a);
    void div(Element &r, const Element &a, const Element &b);
    void exp(Element &r, const Element &base, uint8_t* scalar, unsigned int scalarSize);

    void inline toMontgomery(Element &r, const Element &a) { Fr_rawToMontgomery(r.v, a.v); };
    void inline fromMontgomery(Element &r, const Element &a) { Fr_rawFromMontgomery(r.v, a.v); };
    int inline eq(const Element &a, const Element &b) { return Fr_rawIsEq(a.v, b.v); };
    int inline isZero(const Element &a) { return Fr_rawIsZero(a.v); };

    void toMpz(mpz_t r, const Element &a);
    void fromMpz(Element &a, const mpz_t r);

    int toRprBE(const Element &element, uint8_t *data, int bytes);
    int fromRprBE(Element &element, const uint8_t *data, int bytes);

    int bytes ( void ) { return Fr_N64 * 8; };

    void fromUI(Element &r, unsigned long int v);

    static RawFr field;

};


#endif // __FR_H



