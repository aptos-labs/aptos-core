#include <string>

#include "exp.hpp"
#include "multiexp.hpp"


template <typename BaseField>
class Curve {

    void mulByA(typename BaseField::Element &r, typename BaseField::Element &ab);
public:
    struct Point {
        typename BaseField::Element x;
        typename BaseField::Element y;
        typename BaseField::Element zz;
        typename BaseField::Element zzz;
    };

    struct PointAffine {
        typename BaseField::Element x;
        typename BaseField::Element y;
    };

private: 

    void initCurve(typename BaseField::Element &aa, typename BaseField::Element &ab, typename BaseField::Element &agx, typename BaseField::Element &agy);

    enum TypeOfA { a_is_zero, a_is_one, a_is_negone, a_is_long };
    TypeOfA typeOfA;

    // y^2 = x^3 + a*x + b
    typename BaseField::Element fa;
    typename BaseField::Element fb;
    Point fone;
    Point fzero;
    PointAffine foneAffine;
    PointAffine fzeroAffine;



public:

#ifdef COUNT_OPS
    int cntAddMixed;
    int cntAdd;
    int cntAddAffine;
    int cntDbl;
    int cntEq;
    int cntEqMixed;    
    int cntDblMixed;
    int cntToAffine;
#endif // COUNT_OPS


    BaseField &F;

    Curve(BaseField &aF, typename BaseField::Element &aa, typename BaseField::Element &ab, typename BaseField::Element &agx, typename BaseField::Element &agy);
    Curve(BaseField &aF, std::string as, std::string bs, std::string gxx, std::string gys);

    typename BaseField::Element &a() {return fa; };
    typename BaseField::Element &b() {return fb; };
    Point &one() {return fone; };
    PointAffine &oneAffine() {return foneAffine; };
    Point &zero() {return fzero; };
    PointAffine &zeroAffine() {return fzeroAffine; };

    void add(Point &p3, Point &p1, Point &p2);
    void add(Point &p3, Point &p1, PointAffine &p2);
    void add(Point &p3, PointAffine &p1, PointAffine &p2);
    void add(Point &p3, PointAffine &p1, Point &p2) { add(p3, p2, p1); };
    void add(PointAffine &p3, Point &p1, Point &p2) { Point tmp; add(tmp, p1, p2); copy(p3, tmp); };
    void add(PointAffine &p3, Point &p1, PointAffine &p2) { Point tmp; add(tmp, p1, p2); copy(p3, tmp); };
    void add(PointAffine &p3, PointAffine &p1, PointAffine &p2) { Point tmp; add(tmp, p1, p2); copy(p3, tmp); };
    void add(PointAffine &p3, PointAffine &p1, Point &p2) { Point tmp; add(tmp, p1, p2); copy(p3, tmp); };

    void sub(Point &p3, Point &p1, Point &p2) { Point tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(Point &p3, Point &p1, PointAffine &p2) { PointAffine tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(Point &p3, PointAffine &p1, PointAffine &p2) { PointAffine tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(Point &p3, PointAffine &p1, Point &p2) { Point tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(PointAffine &p3, Point &p1, Point &p2) { Point tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(PointAffine &p3, Point &p1, PointAffine &p2) { PointAffine tmp; neg(tmp, p2); add(p3, p1, tmp); };
    void sub(PointAffine &p3, PointAffine &p1, PointAffine &p2) { PointAffine tmp; neg(tmp, p2); add(p3, p1, tmp); }
    void sub(PointAffine &p3, PointAffine &p1, Point &p2) { Point tmp; neg(tmp, p2); add(p3, p1, tmp); }

    void dbl(Point &r, Point &a);
    void dbl(Point &r, PointAffine &a);
    void dbl(PointAffine &r, Point &a) { Point tmp; dbl(tmp, a); copy(r, tmp); }
    void dbl(PointAffine &r, PointAffine &a) { Point tmp; dbl(tmp, a); copy(r, tmp); }


    void neg(Point &r, Point &a);
    void neg(PointAffine &r, PointAffine &a);
    void neg(Point &r, PointAffine &a);
    void neg(PointAffine &r, Point &a);

    bool eq(Point &p1, Point &p2);
    bool eq(Point &p1, PointAffine &p2);
    bool eq(PointAffine &p1, PointAffine &p2);
    bool eq(PointAffine &p1, Point &p2) { return eq(p2, p1); }

    bool isZero(Point &p1);
    bool isZero(PointAffine &p1);

    std::string toString(Point &r, uint32_t radix = 10);

    void copy(Point &r, Point &a);
    void copy(Point &r, PointAffine &a);
    void copy(PointAffine &r, Point &a);
    void copy(PointAffine &r, PointAffine &a);

    void mulByScalar(Point &r, Point &base, uint8_t* scalar, unsigned int scalarSize) {
        nafMulByScalar<Curve<BaseField>, Point, Point>(*this, r, base, scalar, scalarSize);
    }

    void mulByScalar(Point &r, PointAffine &base, uint8_t* scalar, unsigned int scalarSize) {
        nafMulByScalar<Curve<BaseField>, PointAffine, Point>(*this, r, base, scalar, scalarSize);
    }

    void multiMulByScalar(Point &r, PointAffine *bases, uint8_t* scalars, unsigned int scalarSize, unsigned int n, unsigned int nThreads=0) {
        ParallelMultiexp<Curve<BaseField>> pm(*this);
        pm.multiexp(r, bases, scalars, scalarSize, n, nThreads);
    }
    void multiMulByScalar(Point &r, PointAffine *bases, uint8_t* scalars, unsigned int scalarSize, unsigned int n,
                          uint32_t nx, uint64_t x[],  unsigned int nThreads=0) {
        ParallelMultiexp<Curve<BaseField>> pm(*this);
        pm.multiexp(r, bases, scalars, scalarSize, n, nx, x, nThreads);
    }

#ifdef COUNT_OPS
    void resetCounters();
    void printCounters();
#endif // COUNT_OPS

};



#include "curve.cpp"
