#include "splitparstr.hpp"
#include "assert.h"
#include <sstream>

template <typename BaseField>
F2Field<BaseField>::F2Field(typename BaseField::Element &anr) {
    initField(anr);
}

template <typename BaseField>
F2Field<BaseField>::F2Field(std::string nrs) {

    typename BaseField::Element anr;

    F.fromString(anr, nrs);

    initField(anr);
}

template <typename BaseField>
void F2Field<BaseField>::initField(typename BaseField::Element &anr) {
    F.copy(nr, anr);
    F.copy(fZero.a, F.zero());
    F.copy(fZero.b, F.zero());
    F.copy(fOne.a, F.one());
    F.copy(fOne.b, F.zero());
    F.copy(fNegOne.a, F.negOne());
    F.copy(fNegOne.b, F.zero());

    if (F.isZero(nr)) {
        typeOfNr = nr_is_zero;
    } else if (F.eq(nr, F.one())) {
        typeOfNr = nr_is_one;
    } else if (F.eq(nr, F.negOne())) {
        typeOfNr = nr_is_negone;
    } else {
        typeOfNr = nr_is_long;
    }
}


template <typename BaseField>
void F2Field<BaseField>::fromString(Element &r, std::string s) {

    auto els = splitParStr(s);
    assert(els.size() == 2);

    F.fromString(r.a, els[0]);
    F.fromString(r.b, els[1]);
}

template <typename BaseField>
std::string F2Field<BaseField>::toString(Element &e, uint32_t radix) {
    std::ostringstream stringStream;
    stringStream << "(" << F.toString(e.a, radix) << "," << F.toString(e.b, radix) << ")";
    return stringStream.str();
}

template <typename BaseField>
void inline F2Field<BaseField>::mulByNr(typename BaseField::Element &r, typename BaseField::Element &a) {
    switch (typeOfNr) {
        case nr_is_zero: F.copy(r, F.zero()); break;
        case nr_is_one: F.copy(r, a); break;
        case nr_is_negone: F.neg(r, a); break;
        case nr_is_long: F.mul(r, nr, a);
    }
}

template <typename BaseField>
void F2Field<BaseField>::add(Element &r, Element &a, Element &b) {
    F.add(r.a, a.a, b.a);
    F.add(r.b, a.b, b.b);
}

template <typename BaseField>
void F2Field<BaseField>::sub(Element &r, Element &a, Element &b) {
    F.sub(r.a, a.a, b.a);
    F.sub(r.b, a.b, b.b);
}

template <typename BaseField>
void F2Field<BaseField>::neg(Element &r, Element &a) {
    F.neg(r.a, a.a);
    F.neg(r.b, a.b);
}

template <typename BaseField>
void F2Field<BaseField>::copy(Element &r, Element &a) {
    F.copy(r.a, a.a);
    F.copy(r.b, a.b);
}

template <typename BaseField>
void F2Field<BaseField>::mul(Element &r, Element &e1, Element &e2) {
    typename BaseField::Element aa;
    F.mul(aa, e1.a, e2.a);
    typename BaseField::Element bb;
    F.mul(bb, e1.b, e2.b);

    typename BaseField::Element bbr;
    mulByNr(bbr, bb);

    typename BaseField::Element sum1, sum2;
    F.add(sum1, e1.a, e1.b);
    F.add(sum2, e2.a, e2.b);

    F.add(r.a, aa, bbr);

    F.mul(r.b, sum1, sum2 );
    F.sub(r.b, r.b, aa);
    F.sub(r.b, r.b, bb);
}

template <typename BaseField>
void F2Field<BaseField>::square(Element &r, Element &e1) {
    typename BaseField::Element ab;
    typename BaseField::Element tmp1, tmp2;

    if (typeOfNr == nr_is_negone) {
        F.mul(ab, e1.a, e1.b);

        F.add(tmp1, e1.a, e1.b);
        F.sub(tmp2, e1.a, e1.b);

        F.mul(r.a, tmp1, tmp2);
        F.add(r.b, ab, ab);
    } else {
        F.mul(ab, e1.a, e1.b);

        F.add(tmp1, e1.a, e1.b);
        mulByNr(tmp2, e1.b);
        F.add(tmp2, e1.a, tmp1);

        F.mul(tmp1, tmp1, tmp2);

        mulByNr(tmp2, ab);
        F.add(tmp2, ab, tmp2);

        F.sub(r.a, tmp1, tmp2);
        F.add(r.b, ab, ab);
    }
}

template <typename BaseField>
void F2Field<BaseField>::inv(Element &r, Element &e1) {
    typename BaseField::Element t0, t1, t2, t3;
    F.square(t0, e1.a);
    F.square(t1, e1.b);
    mulByNr(t2, t1);
    F.sub(t2, t0, t2);
    F.inv(t3, t2);
    F.mul(r.a, e1.a, t3);
    F.mul(r.b, e1.b, t3);
    F.neg(r.b, r.b);
}

template <typename BaseField>
void F2Field<BaseField>::div(Element &r, Element &e1, Element &e2) {
    Element tmp;
    inv(tmp, e2);
    mul(r, e1, tmp);
}

template <typename BaseField>
bool F2Field<BaseField>::isZero(Element &a) {
    return F.isZero(a.a) && F.isZero(a.b);
}

template <typename BaseField>
bool F2Field<BaseField>::eq(Element &a, Element &b) {
    return F.eq(a.a, b.a) && F.eq(a.b, b.b);
}
