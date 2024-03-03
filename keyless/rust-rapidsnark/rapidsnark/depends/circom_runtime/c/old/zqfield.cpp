#include "zqfield.h"

ZqField::ZqField(PBigInt ap) {
    mpz_init_set(p, *ap);
    mpz_init_set_ui(zero, 0);
    mpz_init_set_ui(one, 1);
    nBits = mpz_sizeinbase (p, 2);
    mpz_init(mask);
    mpz_mul_2exp(mask, one, nBits-1);
    mpz_sub(mask, mask, one);
}

ZqField::~ZqField() {
    mpz_clear(p);
    mpz_clear(zero);
    mpz_clear(one);
}

void ZqField::add(PBigInt r, PBigInt a, PBigInt b) {
    mpz_add(*r,*a,*b);
    if (mpz_cmp(*r, p) >= 0) {
        mpz_sub(*r, *r, p);
    }
}

void ZqField::sub(PBigInt r, PBigInt a, PBigInt b) {
    if (mpz_cmp(*a, *b) >= 0) {
        mpz_sub(*r, *a, *b);
    } else {
        mpz_sub(*r, *b, *a);
        mpz_sub(*r, p, *r);
    }
}

void ZqField::neg(PBigInt r, PBigInt a) {
    if (mpz_sgn(*a) > 0) {
        mpz_sub(*r, p, *a);
    } else {
        mpz_set(*r, *a);
    }
}

void ZqField::mul(PBigInt r, PBigInt a, PBigInt b) {
    mpz_t tmp;
    mpz_init(tmp);
    mpz_mul(tmp,*a,*b);
    mpz_fdiv_r(*r, tmp, p);
    mpz_clear(tmp);
}

void ZqField::div(PBigInt r, PBigInt a, PBigInt b) {
    mpz_t tmp;
    mpz_init(tmp);
    mpz_invert(tmp, *b, p);
    mpz_mul(tmp,*a,tmp);
    mpz_fdiv_r(*r, tmp, p);
    mpz_clear(tmp);
}

void ZqField::idiv(PBigInt r, PBigInt a, PBigInt b) {
    mpz_fdiv_q(*r, *a, *b);
}

void ZqField::mod(PBigInt r, PBigInt a, PBigInt b) {
    mpz_fdiv_r(*r, *a, *b);
}

void ZqField::pow(PBigInt r, PBigInt a, PBigInt b) {
    mpz_powm(*r, *a, *b, p);
}

void ZqField::lt(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c<0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::eq(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c==0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::gt(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c>0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::leq(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c<=0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::geq(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c>=0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::neq(PBigInt r, PBigInt a, PBigInt b) {
    int c = mpz_cmp(*a, *b);
    if (c!=0) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::land(PBigInt r, PBigInt a, PBigInt b) {
    if (mpz_sgn(*a) && mpz_sgn(*b)) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::lor(PBigInt r, PBigInt a, PBigInt b) {
    if (mpz_sgn(*a) || mpz_sgn(*b)) {
        mpz_set(*r, one);
    } else {
        mpz_set(*r, zero);
    }
}

void ZqField::lnot(PBigInt r, PBigInt a) {
    if (mpz_sgn(*a)) {
        mpz_set(*r, zero);
    } else {
        mpz_set(*r, one);
    }
}

int ZqField::isTrue(PBigInt a) {
    return mpz_sgn(*a);
}

void ZqField::copyn(PBigInt a, PBigInt b, int n) {
    for (int i=0;i<n; i++) mpz_set(a[i], b[i]);
}

void ZqField::band(PBigInt r, PBigInt a, PBigInt b) {
    mpz_and(*r, *a, *b);
    mpz_and(*r, *r, mask);
}

void ZqField::bor(PBigInt r, PBigInt a, PBigInt b) {
    mpz_ior(*r, *a, *b);
    mpz_and(*r, *r, mask);
}

void ZqField::bxor(PBigInt r, PBigInt a, PBigInt b) {
    mpz_xor(*r, *a, *b);
    mpz_and(*r, *r, mask);
}

void ZqField::bnot(PBigInt r, PBigInt a) {
    mpz_xor(*r, *a, mask);
    mpz_and(*r, *r, mask);
}

void ZqField::shl(PBigInt r, PBigInt a, PBigInt b) {
    if (mpz_cmp_ui(*b, nBits) >= 0) {
        mpz_set(*r, zero);
    } else {
        mpz_mul_2exp(*r, *a, mpz_get_ui(*b));
        mpz_and(*r, *r, mask);
    }
}

void ZqField::shr(PBigInt r, PBigInt a, PBigInt b) {
    if (mpz_cmp_ui(*b, nBits) >= 0) {
        mpz_set(*r, zero);
    } else {
        mpz_tdiv_q_2exp(*r, *a, mpz_get_ui(*b));
        mpz_and(*r, *r, mask);
    }
}

int ZqField::toInt(PBigInt a) {
     return mpz_get_si (*a);
}

