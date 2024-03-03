#ifndef ZQFIELD_H
#define ZQFIELD_H

#include "circom.h"

class ZqField {
public:
    BigInt p;
    BigInt one;
    BigInt zero;
    size_t nBits;
    BigInt mask;
    ZqField(PBigInt ap);
    ~ZqField();

    void copyn(PBigInt a, PBigInt b, int n);

    void add(PBigInt r,PBigInt a, PBigInt b);
    void sub(PBigInt r,PBigInt a, PBigInt b);
    void neg(PBigInt r,PBigInt a);
    void mul(PBigInt r,PBigInt a, PBigInt b);
    void div(PBigInt r,PBigInt a, PBigInt b);
    void idiv(PBigInt r,PBigInt a, PBigInt b);
    void mod(PBigInt r,PBigInt a, PBigInt b);
    void pow(PBigInt r,PBigInt a, PBigInt b);

    void lt(PBigInt r, PBigInt a, PBigInt b);
    void eq(PBigInt r, PBigInt a, PBigInt b);
    void gt(PBigInt r, PBigInt a, PBigInt b);
    void leq(PBigInt r, PBigInt a, PBigInt b);
    void geq(PBigInt r, PBigInt a, PBigInt b);
    void neq(PBigInt r, PBigInt a, PBigInt b);

    void land(PBigInt r, PBigInt a, PBigInt b);
    void lor(PBigInt r, PBigInt a, PBigInt b);
    void lnot(PBigInt r, PBigInt a);

    void band(PBigInt r, PBigInt a, PBigInt b);
    void bor(PBigInt r, PBigInt a, PBigInt b);
    void bxor(PBigInt r, PBigInt a, PBigInt b);
    void bnot(PBigInt r, PBigInt a);
    void shl(PBigInt r, PBigInt a, PBigInt b);
    void shr(PBigInt r, PBigInt a, PBigInt b);

    int isTrue(PBigInt a);
    int toInt(PBigInt a);
};

#endif // ZQFIELD_H
