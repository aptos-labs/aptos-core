#include <string>

template <typename BaseField>
class F2Field {

public:
    struct Element {
        typename BaseField::Element a;
        typename BaseField::Element b;
    };

    BaseField F;
private:
    enum TypeOfNr { nr_is_zero, nr_is_one, nr_is_negone, nr_is_long };
    TypeOfNr typeOfNr;

    typename BaseField::Element nr;

    Element fOne;
    Element fZero;
    Element fNegOne;

    void mulByNr(typename BaseField::Element &r, typename BaseField::Element &ab);

    void initField(typename BaseField::Element &anr);
public:

    F2Field(typename BaseField::Element &anr);
    F2Field(std::string nrs);

    Element &zero() { return fZero; };
    Element &one() { return fOne; };
    Element &negOne() { return fNegOne; };

    void copy(Element &r, Element &a);
    void add(Element &r, Element &a, Element &b);
    void sub(Element &r, Element &a, Element &b);
    void neg(Element &r, Element &a);
    void mul(Element &r, Element &a, Element &b);
    void square(Element &r, Element &a);
    void inv(Element &r, Element &a);
    void div(Element &r, Element &a, Element &b);
    bool isZero(Element &a);
    bool eq(Element &a, Element &b);

    void fromString(Element &r, std::string s);
    std::string toString(Element &a, uint32_t radix = 10);

};

#include "f2field.cpp"