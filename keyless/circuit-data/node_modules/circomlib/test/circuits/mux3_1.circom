pragma circom 2.0.0;

include "../../circuits/mux3.circom";
include "../../circuits/bitify.circom";


template Constants() {
    var i;
    signal output out[8];

    out[0] <== 37;
    out[1] <== 47;
    out[2] <== 53;
    out[3] <== 71;
    out[4] <== 89;
    out[5] <== 107;
    out[6] <== 163;
    out[7] <== 191;
}

template Main() {
    var i;
    signal input selector;//private
    signal output out;

    component mux = Mux3();
    component n2b = Num2Bits(3);
    component cst = Constants();

    selector ==> n2b.in;
    for (i=0; i<3; i++) {
        n2b.out[i] ==> mux.s[i];
    }
    for (i=0; i<8; i++) {
        cst.out[i] ==> mux.c[i];
    }

    mux.out ==> out;
}

component main = Main();
