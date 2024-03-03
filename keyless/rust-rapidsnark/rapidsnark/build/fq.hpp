#ifndef __FQ_H
#define __FQ_H

#include "fq_element.hpp"
#include <cstdint>
#include <string>
#include <gmp.h>

#ifdef __APPLE__
#include <sys/types.h> // typedef unsigned int uint;
#endif // __APPLE__

extern FqElement Fq_q;
extern FqElement Fq_R2;
extern FqElement Fq_R3;
extern FqRawElement Fq_rawq;
extern FqRawElement Fq_rawR3;

#ifdef USE_ASM

#if defined(ARCH_X86_64)

extern "C" void Fq_copy(PFqElement r, PFqElement a);
extern "C" void Fq_copyn(PFqElement r, PFqElement a, int n);
extern "C" void Fq_add(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_sub(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_neg(PFqElement r, PFqElement a);
extern "C" void Fq_mul(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_square(PFqElement r, PFqElement a);
extern "C" void Fq_band(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_bor(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_bxor(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_bnot(PFqElement r, PFqElement a);
extern "C" void Fq_shl(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_shr(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_eq(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_neq(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_lt(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_gt(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_leq(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_geq(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_land(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_lor(PFqElement r, PFqElement a, PFqElement b);
extern "C" void Fq_lnot(PFqElement r, PFqElement a);
extern "C" void Fq_toNormal(PFqElement r, PFqElement a);
extern "C" void Fq_toLongNormal(PFqElement r, PFqElement a);
extern "C" void Fq_toMontgomery(PFqElement r, PFqElement a);

extern "C" int Fq_isTrue(PFqElement pE);
extern "C" int Fq_toInt(PFqElement pE);

extern "C" void Fq_rawCopy(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawSwap(FqRawElement pRawResult, FqRawElement pRawA);
extern "C" void Fq_rawAdd(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" void Fq_rawSub(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" void Fq_rawNeg(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawMMul(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" void Fq_rawMSquare(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawMMul1(FqRawElement pRawResult, const FqRawElement pRawA, uint64_t pRawB);
extern "C" void Fq_rawToMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
extern "C" void Fq_rawFromMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
extern "C" int Fq_rawIsEq(const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" int Fq_rawIsZero(const FqRawElement pRawB);
extern "C" void Fq_rawShl(FqRawElement r, FqRawElement a, uint64_t b);
extern "C" void Fq_rawShr(FqRawElement r, FqRawElement a, uint64_t b);

extern "C" void Fq_fail();

#elif defined(ARCH_ARM64)

           void Fq_copy(PFqElement r, PFqElement a);
           void Fq_mul(PFqElement r, PFqElement a, PFqElement b);
           void Fq_toNormal(PFqElement r, PFqElement a);

           void Fq_toLongNormal(PFqElement r, PFqElement a);
           int  Fq_isTrue(PFqElement pE);
           void Fq_copyn(PFqElement r, PFqElement a, int n);
           void Fq_lt(PFqElement r, PFqElement a, PFqElement b);
           int  Fq_toInt(PFqElement pE);
           void Fq_shr(PFqElement r, PFqElement a, PFqElement b);
           void Fq_shl(PFqElement r, PFqElement a, PFqElement b);
           void Fq_band(PFqElement r, PFqElement a, PFqElement b);
           void Fq_bor(PFqElement r, PFqElement a, PFqElement b);
           void Fq_bxor(PFqElement r, PFqElement a, PFqElement b);
           void Fq_bnot(PFqElement r, PFqElement a);
           void Fq_sub(PFqElement r, PFqElement a, PFqElement b);
           void Fq_eq(PFqElement r, PFqElement a, PFqElement b);
           void Fq_neq(PFqElement r, PFqElement a, PFqElement b);
           void Fq_add(PFqElement r, PFqElement a, PFqElement b);
           void Fq_gt(PFqElement r, PFqElement a, PFqElement b);
           void Fq_leq(PFqElement r, PFqElement a, PFqElement b);
           void Fq_geq(PFqElement r, PFqElement a, PFqElement b);
           void Fq_lor(PFqElement r, PFqElement a, PFqElement b);
           void Fq_lnot(PFqElement r, PFqElement a);
           void Fq_land(PFqElement r, PFqElement a, PFqElement b);
           void Fq_neg(PFqElement r, PFqElement a);
           void Fq_toMontgomery(PFqElement r, PFqElement a);
           void Fq_square(PFqElement r, PFqElement a);

extern "C" void Fq_rawCopy(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawSwap(FqRawElement pRawResult, FqRawElement pRawA);
extern "C" void Fq_rawAdd(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" void Fq_rawSub(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" void Fq_rawNeg(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawMMul(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
           void Fq_rawMSquare(FqRawElement pRawResult, const FqRawElement pRawA);
extern "C" void Fq_rawMMul1(FqRawElement pRawResult, const FqRawElement pRawA, uint64_t pRawB);
           void Fq_rawToMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
extern "C" void Fq_rawFromMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
extern "C" int  Fq_rawIsEq(const FqRawElement pRawA, const FqRawElement pRawB);
extern "C" int  Fq_rawIsZero(const FqRawElement pRawB);
           void Fq_rawZero(FqRawElement pRawResult);
extern "C" void Fq_rawCopyS2L(FqRawElement pRawResult, int64_t val);
extern "C" void Fq_rawAddLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
extern "C" void Fq_rawSubSL(FqRawElement pRawResult, uint64_t rawA, FqRawElement pRawB);
extern "C" void Fq_rawSubLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
extern "C" void Fq_rawNegLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
extern "C" int  Fq_rawCmp(FqRawElement pRawA, FqRawElement pRawB);
extern "C" void Fq_rawAnd(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
extern "C" void Fq_rawOr(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
extern "C" void Fq_rawXor(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
extern "C" void Fq_rawShl(FqRawElement r, FqRawElement a, uint64_t b);
extern "C" void Fq_rawShr(FqRawElement r, FqRawElement a, uint64_t b);
extern "C" void Fq_rawNot(FqRawElement pRawResult, FqRawElement pRawA);
extern "C" void Fq_rawSubRegular(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);

void Fq_fail();
void Fq_longErr();

#endif

#else


void Fq_copy(PFqElement r, PFqElement a);
void Fq_mul(PFqElement r, PFqElement a, PFqElement b);
void Fq_toNormal(PFqElement r, PFqElement a);

void Fq_toLongNormal(PFqElement r, PFqElement a);
int Fq_isTrue(PFqElement pE);
void Fq_copyn(PFqElement r, PFqElement a, int n);
void Fq_lt(PFqElement r, PFqElement a, PFqElement b);
int Fq_toInt(PFqElement pE);
void Fq_shl(PFqElement r, PFqElement a, PFqElement b);
void Fq_shr(PFqElement r, PFqElement a, PFqElement b);
void Fq_band(PFqElement r, PFqElement a, PFqElement b);
void Fq_bor(PFqElement r, PFqElement a, PFqElement b);
void Fq_bxor(PFqElement r, PFqElement a, PFqElement b);
void Fq_bnot(PFqElement r, PFqElement a);
void Fq_sub(PFqElement r, PFqElement a, PFqElement b);
void Fq_eq(PFqElement r, PFqElement a, PFqElement b);
void Fq_neq(PFqElement r, PFqElement a, PFqElement b);
void Fq_add(PFqElement r, PFqElement a, PFqElement b);
void Fq_gt(PFqElement r, PFqElement a, PFqElement b);
void Fq_leq(PFqElement r, PFqElement a, PFqElement b);
void Fq_geq(PFqElement r, PFqElement a, PFqElement b);
void Fq_lor(PFqElement r, PFqElement a, PFqElement b);
void Fq_lnot(PFqElement r, PFqElement a);
void Fq_land(PFqElement r, PFqElement a, PFqElement b);
void Fq_neg(PFqElement r, PFqElement a);
void Fq_toMontgomery(PFqElement r, PFqElement a);
void Fq_square(PFqElement r, PFqElement a);

void Fq_rawCopy(FqRawElement pRawResult, const FqRawElement pRawA);
void Fq_rawSwap(FqRawElement pRawResult, FqRawElement pRawA);
void Fq_rawAdd(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
void Fq_rawSub(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
void Fq_rawNeg(FqRawElement pRawResult, const FqRawElement pRawA);
void Fq_rawMMul(FqRawElement pRawResult, const FqRawElement pRawA, const FqRawElement pRawB);
void Fq_rawMSquare(FqRawElement pRawResult, const FqRawElement pRawA);
void Fq_rawMMul1(FqRawElement pRawResult, const FqRawElement pRawA, uint64_t pRawB);
void Fq_rawToMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
void Fq_rawFromMontgomery(FqRawElement pRawResult, const FqRawElement &pRawA);
int Fq_rawIsEq(const FqRawElement pRawA, const FqRawElement pRawB);
int Fq_rawIsZero(const FqRawElement pRawB);
void Fq_rawZero(FqRawElement pRawResult);
void Fq_rawCopyS2L(FqRawElement pRawResult, int64_t val);
void Fq_rawAddLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
void Fq_rawSubSL(FqRawElement pRawResult, uint64_t rawA, FqRawElement pRawB);
void Fq_rawSubLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
void Fq_rawNegLS(FqRawElement pRawResult, FqRawElement pRawA, uint64_t rawB);
int  Fq_rawCmp(FqRawElement pRawA, FqRawElement pRawB);
void Fq_rawAnd(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
void Fq_rawOr(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
void Fq_rawXor(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);
void Fq_rawShl(FqRawElement r, FqRawElement a, uint64_t b);
void Fq_rawShr(FqRawElement r, FqRawElement a, uint64_t b);
void Fq_rawNot(FqRawElement pRawResult, FqRawElement pRawA);
void Fq_rawSubRegular(FqRawElement pRawResult, FqRawElement pRawA, FqRawElement pRawB);

void Fq_fail();
void Fq_longErr();

#endif

// Pending functions to convert

void Fq_str2element(PFqElement pE, char const*s, uint base);
char *Fq_element2str(PFqElement pE);
void Fq_idiv(PFqElement r, PFqElement a, PFqElement b);
void Fq_mod(PFqElement r, PFqElement a, PFqElement b);
void Fq_inv(PFqElement r, PFqElement a);
void Fq_div(PFqElement r, PFqElement a, PFqElement b);
void Fq_pow(PFqElement r, PFqElement a, PFqElement b);

class RawFq {

public:
    const static int N64 = Fq_N64;
    const static int MaxBits = 254;


    struct Element {
        FqRawElement v;
    };

private:
    Element fZero;
    Element fOne;
    Element fNegOne;

public:

    RawFq();
    ~RawFq();

    const Element &zero() { return fZero; };
    const Element &one() { return fOne; };
    const Element &negOne() { return fNegOne; };
    Element set(int value);
    void set(Element &r, int value);

    void fromString(Element &r, const std::string &n, uint32_t radix = 10);
    std::string toString(const Element &a, uint32_t radix = 10);

    void inline copy(Element &r, const Element &a) { Fq_rawCopy(r.v, a.v); };
    void inline swap(Element &a, Element &b) { Fq_rawSwap(a.v, b.v); };
    void inline add(Element &r, const Element &a, const Element &b) { Fq_rawAdd(r.v, a.v, b.v); };
    void inline sub(Element &r, const Element &a, const Element &b) { Fq_rawSub(r.v, a.v, b.v); };
    void inline mul(Element &r, const Element &a, const Element &b) { Fq_rawMMul(r.v, a.v, b.v); };

    Element inline add(const Element &a, const Element &b) { Element r; Fq_rawAdd(r.v, a.v, b.v); return r;};
    Element inline sub(const Element &a, const Element &b) { Element r; Fq_rawSub(r.v, a.v, b.v); return r;};
    Element inline mul(const Element &a, const Element &b) { Element r; Fq_rawMMul(r.v, a.v, b.v); return r;};

    Element inline neg(const Element &a) { Element r; Fq_rawNeg(r.v, a.v); return r; };
    Element inline square(const Element &a) { Element r; Fq_rawMSquare(r.v, a.v); return r; };

    Element inline add(int a, const Element &b) { return add(set(a), b);};
    Element inline sub(int a, const Element &b) { return sub(set(a), b);};
    Element inline mul(int a, const Element &b) { return mul(set(a), b);};

    Element inline add(const Element &a, int b) { return add(a, set(b));};
    Element inline sub(const Element &a, int b) { return sub(a, set(b));};
    Element inline mul(const Element &a, int b) { return mul(a, set(b));};

    void inline mul1(Element &r, const Element &a, uint64_t b) { Fq_rawMMul1(r.v, a.v, b); };
    void inline neg(Element &r, const Element &a) { Fq_rawNeg(r.v, a.v); };
    void inline square(Element &r, const Element &a) { Fq_rawMSquare(r.v, a.v); };
    void inv(Element &r, const Element &a);
    void div(Element &r, const Element &a, const Element &b);
    void exp(Element &r, const Element &base, uint8_t* scalar, unsigned int scalarSize);

    void inline toMontgomery(Element &r, const Element &a) { Fq_rawToMontgomery(r.v, a.v); };
    void inline fromMontgomery(Element &r, const Element &a) { Fq_rawFromMontgomery(r.v, a.v); };
    int inline eq(const Element &a, const Element &b) { return Fq_rawIsEq(a.v, b.v); };
    int inline isZero(const Element &a) { return Fq_rawIsZero(a.v); };

    void toMpz(mpz_t r, const Element &a);
    void fromMpz(Element &a, const mpz_t r);

    int toRprBE(const Element &element, uint8_t *data, int bytes);
    int fromRprBE(Element &element, const uint8_t *data, int bytes);

    int bytes ( void ) { return Fq_N64 * 8; };

    void fromUI(Element &r, unsigned long int v);

    static RawFq field;

};


#endif // __FQ_H



