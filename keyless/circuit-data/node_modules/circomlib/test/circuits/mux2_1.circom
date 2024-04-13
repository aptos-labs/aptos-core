pragma circom 2.0.0;

include "../../circuits/mux2.circom";
include "../../circuits/bitify.circom";


template Constants() {
    var i;
    signal output out[4];

    out[0] <== 37;
    out[1] <== 47;
    out[2] <== 53;
    out[3] <== 71;
}

template Main() {
    var i;
    signal input selector;//private
    signal output out;

    component mux = Mux2();
    component n2b = Num2Bits(2);
    component cst = Constants();

    selector ==> n2b.in;
    for (i=0; i<2; i++) {
        n2b.out[i] ==> mux.s[i];
    }
    for (i=0; i<4; i++) {
        cst.out[i] ==> mux.c[i];
    }

    mux.out ==> out;
}

component main = Main();
