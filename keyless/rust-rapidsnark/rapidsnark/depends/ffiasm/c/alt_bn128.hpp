#ifndef ALT_BN128_HPP
#define ALT_BN128_HPP

#include "fq.hpp"
#include "fr.hpp"
#include "f2field.hpp"
#include "curve.hpp"
#include <string>
namespace AltBn128 {

    typedef RawFq::Element F1Element;
    typedef F2Field<RawFq>::Element F2Element;
    typedef RawFr::Element FrElement;
    typedef Curve<RawFq>::Point G1Point;
    typedef Curve<RawFq>::PointAffine G1PointAffine;
    typedef Curve< F2Field<RawFq> >::Point G2Point;
    typedef Curve< F2Field<RawFq> >::PointAffine G2PointAffine;

    extern RawFq F1;
    extern F2Field<RawFq> F2;
    extern RawFr Fr;
    extern Curve<RawFq> G1;
    extern Curve< F2Field<RawFq> > G2;

    class Engine {
    public:

        typedef RawFq F1;
        typedef F2Field<RawFq> F2;
        typedef RawFr Fr;
        typedef Curve<RawFq> G1;
        typedef Curve< F2Field<RawFq> > G2;

        F1 f1;
        F2 f2;
        Fr fr;
        G1 g1;
        G2 g2;

        Engine() : 
            f1(), 
            f2("-1"), 
            fr(), 
            g1(f1, "0", "3", "1", "2"), 
            g2(
                f2,
                "0,0", 
                "19485874751759354771024239261021720505790618469301721065564631296452457478373, 266929791119991161246907387137283842545076965332900288569378510910307636690",
                "10857046999023057135944570762232829481370756359578518086990519993285655852781, 11559732032986387107991004021392285783925812861821192530917403151452391805634",
                "8495653923123431417604973247489272438418190587263600148770280649306958101930, 4082367875863433681332203403145435568316851327593401208105741076214120093531"
            ) {}

        typedef F1::Element F1Element;
        typedef F2::Element F2Element;
        typedef Fr::Element FrElement;
        typedef G1::Point G1Point;
        typedef G1::PointAffine G1PointAffine;
        typedef G2::Point G2Point;
        typedef G2::PointAffine G2PointAffine;

        static Engine engine;
    };

}  // Namespace

#endif