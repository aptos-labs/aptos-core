#ifndef __FR_H
#define __FR_H

#include <stdint.h>
#define Fr_N64 4
#define Fr_SHORT 0x00000000
#define Fr_LONG 0x80000000
#define Fr_LONGMONTGOMERY 0xC0000000
typedef uint64_t FrRawElement[Fr_N64];
typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    FrRawElement longVal;
} FrElement;
typedef FrElement *PFrElement;
extern FrElement Fr_q;
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
extern "C" void Fr_eq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_neq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lt(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_gt(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_leq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_geq(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_land(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lor(PFrElement r, PFrElement a, PFrElement b);
extern "C" void Fr_lnot(PFrElement r, PFrElement a);
extern "C" void Fr_toNormal(PFrElement pE);
extern "C" void Fr_toLongNormal(PFrElement pE);
extern "C" void Fr_toMontgomery(PFrElement pE);

extern "C" int Fr_isTrue(PFrElement pE);
extern "C" int Fr_toInt(PFrElement pE);

extern "C" void Fr_rawCopy(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawAdd(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawSub(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawNeg(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawMMul(FrRawElement pRawResult, FrRawElement pRawA, FrRawElement pRawB);
extern "C" void Fr_rawMSquare(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawMMul1(FrRawElement pRawResult, FrRawElement pRawA, uint64_t pRawB);
extern "C" void Fr_rawToMontgomery(FrRawElement pRawResult, FrRawElement pRawA);
extern "C" void Fr_rawFromMontgomery(FrRawElement pRawResult, FrRawElement pRawA);

extern "C" void Fr_fail();

extern FrElement Fr_q;

// Pending functions to convert

void Fr_str2element(PFrElement pE, char const*s);
char *Fr_element2str(PFrElement pE);
void Fr_idiv(PFrElement r, PFrElement a, PFrElement b);
void Fr_mod(PFrElement r, PFrElement a, PFrElement b);
void Fr_inv(PFrElement r, PFrElement a);
void Fr_div(PFrElement r, PFrElement a, PFrElement b);
void Fr_shl(PFrElement r, PFrElement a, PFrElement b);
void Fr_shr(PFrElement r, PFrElement a, PFrElement b);
void Fr_pow(PFrElement r, PFrElement a, PFrElement b);


void Fr_init();

class RawFr {

public:
    RawFr();
    ~RawFr();

    typedef uint64_t Element[Fr_N64];

    void fromString(Element r, char const *n);
    char *toString(Element a);

    void inline copy(Element r, Element a) { Fr_rawCopy(r, a); };
    void inline swap(Element a, Element b) { Fr_rawSwap(r, a); };
    void inline add(Element r, Element a, Element b) { Fr_rawAdd(r, a, b); };
    void inline mul(Element r, Element a, Element b) { Fr_rawMMul(r, a, b); };
    void inline mul1(Element r, Element a, uint64_t b) { Fr_rawMMul1(r, a, b); };
    void inline neg(Element r, Element a) { Fr_rawNeg(r, a); };
    void inline square(Element r, Element a) { Fr_rawMSquare(r, a); };
    void inline toMontgomery(Element r, Element a) { Fr_rawToMontgomery(r, a); };
    void inline fromMontgomery(Element r, Element a) { Fr_rawFromMontgomery(r, a); };
};


#endif // __FR_H



