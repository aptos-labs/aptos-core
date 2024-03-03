#include "fr.hpp"
#include <stdio.h>
#include <stdlib.h>
#include <gmp.h>
#include <assert.h>


bool initialized = false;
mpz_t q;
mpz_t zero;
mpz_t one;
mpz_t mask;
size_t nBits;


void Fr_toMpz(mpz_t r, PFrElement pE) {
    Fr_toNormal(pE);
    if (!(pE->type & Fr_LONG)) {
        mpz_set_si(r, pE->shortVal);
        if (pE->shortVal<0) {
            mpz_add(r, r, q);
        }
    } else {
        Fr_toNormal(pE);
        mpz_import(r, Fr_N64, -1, 8, -1, 0, (const void *)pE->longVal);
    }
}

void Fr_fromMpz(PFrElement pE, mpz_t v) {
    if (mpz_fits_sint_p(v)) {
        pE->type = Fr_SHORT;
        pE->shortVal = mpz_get_si(v);
    } else {
        pE->type = Fr_LONG;
        for (int i=0; i<Fr_N64; i++) pE->longVal[i] = 0;
        mpz_export((void *)(pE->longVal), NULL, -1, 8, -1, 0, v);
    }
}


void Fr_init() {
    if (initialized) return;
    initialized = true;
    mpz_init(q);
    mpz_import(q, Fr_N64, -1, 8, -1, 0, (const void *)Fr_q.longVal);
    mpz_init_set_ui(zero, 0);
    mpz_init_set_ui(one, 1);
    nBits = mpz_sizeinbase (q, 2);
    mpz_init(mask);
    mpz_mul_2exp(mask, one, nBits);
    mpz_sub(mask, mask, one);
}

void Fr_str2element(PFrElement pE, char const *s) {
    mpz_t mr;
    mpz_init_set_str(mr, s, 10);
    Fr_fromMpz(pE, mr);
}

char *Fr_element2str(PFrElement pE) {
    mpz_t r;
    if (!(pE->type & Fr_LONG)) {
        if (pE->shortVal>=0) {
            char *r = new char[32];
            sprintf(r, "%d", pE->shortVal);
            return r;
        } else {
            mpz_init_set_si(r, pE->shortVal);
            mpz_add(r, r, q);
        }
    } else {
        Fr_toNormal(pE);
        mpz_init(r);
        mpz_import(r, Fr_N64, -1, 8, -1, 0, (const void *)pE->longVal);
    }
    char *res = mpz_get_str (0, 10, r);
    mpz_clear(r);
    return res;
}

void Fr_idiv(PFrElement r, PFrElement a, PFrElement b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    // char *s1 = mpz_get_str (0, 10, ma);
    // printf("s1 %s\n", s1);
    Fr_toMpz(mb, b);
    // char *s2 = mpz_get_str (0, 10, mb);
    // printf("s2 %s\n", s2);
    mpz_fdiv_q(mr, ma, mb);
    // char *sr = mpz_get_str (0, 10, mr);
    // printf("r %s\n", sr);
    Fr_fromMpz(r, mr);
}

void Fr_mod(PFrElement r, PFrElement a, PFrElement b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    Fr_toMpz(mb, b);
    mpz_fdiv_r(mr, ma, mb);
    Fr_fromMpz(r, mr);
}

void Fr_shl(PFrElement r, PFrElement a, PFrElement b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    Fr_toMpz(mb, b);
    if (mpz_cmp_ui(mb, nBits) < 0) {
        mpz_mul_2exp(mr, ma, mpz_get_ui(mb));
        mpz_and(mr, mr, mask);
        if (mpz_cmp(mr, q) >= 0) {
            mpz_sub(mr, mr, q);
        }
    } else {
        mpz_sub(mb, q, mb);
        if (mpz_cmp_ui(mb, nBits) < 0) {
            mpz_tdiv_q_2exp(mr, ma, mpz_get_ui(mb));
        } else {
            mpz_set(mr, zero);
        }
    }
    Fr_fromMpz(r, mr);
}

void Fr_shr(PFrElement r, PFrElement a, PFrElement b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    Fr_toMpz(mb, b);
    if (mpz_cmp_ui(mb, nBits) < 0) {
        mpz_tdiv_q_2exp(mr, ma, mpz_get_ui(mb));
    } else {
        mpz_sub(mb, q, mb);
        if (mpz_cmp_ui(mb, nBits) < 0) {
            mpz_mul_2exp(mr, ma, mpz_get_ui(mb));
            mpz_and(mr, mr, mask);
            if (mpz_cmp(mr, q) >= 0) {
                mpz_sub(mr, mr, q);
            }
        } else {
            mpz_set(mr, zero);
        }
    }
    Fr_fromMpz(r, mr);
}


void Fr_pow(PFrElement r, PFrElement a, PFrElement b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    Fr_toMpz(mb, b);
    mpz_powm(mr, ma, mb, q);
    Fr_fromMpz(r, mr);
}

void Fr_inv(PFrElement r, PFrElement a) {
    mpz_t ma;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mr);

    Fr_toMpz(ma, a);
    mpz_invert(mr, ma, q);
    Fr_fromMpz(r, mr);
}

void Fr_div(PFrElement r, PFrElement a, PFrElement b) {
    FrElement tmp;
    Fr_inv(&tmp, b);
    Fr_mul(r, a, &tmp);
}

void Fr_fail() {
    assert(false);
}


RawFr::RawFr() {
    Fr_init();
}

RawFr::~RawFr() {
}

void RawFr::fromString(Element r, char const *s) {
    mpz_t mr;
    mpz_init_set_str(mr, s, 10);
    for (int i=0; i<Fr_N64; i++) r[i] = 0;
    mpz_export((void *)r, NULL, -1, 8, -1, 0, mr);
    Fr_rawToMontgomery(r,r);
}


char *RawFr::toString(Element a) {
    Element tmp;
    mpz_t r;
    Fr_rawFromMontgomery(tmp, a);
    mpz_init(r);
    mpz_import(r, Fr_N64, -1, 8, -1, 0, (const void *)tmp);
    char *res = mpz_get_str (0, 10, r);
    mpz_clear(r);
    return res;
}
    




