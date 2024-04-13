pragma circom 2.0.0;

include "../../circuits/mux4.circom";
include "../../circuits/bitify.circom";


template Constants() {
    var i;
    signal output out[16];

    out[0] <== 123;
    out[1] <== 456;
    out[2] <== 789;
    out[3] <== 012;
    out[4] <== 111;
    out[5] <== 222;
    out[6] <== 333;
    out[7] <== 4546;
    out[8] <== 134523;
    out[9] <== 44356;
    out[10] <== 15623;
    out[11] <== 4566;
    out[12] <== 1223;
    out[13] <== 4546;
    out[14] <== 4256;
    out[15] <== 4456;

/*
    for (i=0;i<16; i++) {
        out[i] <== i*2+100;
    }
*/

}

template Main() {
    var i;
    signal input selector;//private
    signal output out;

    component mux = Mux4();
    component n2b = Num2Bits(4);
    component cst = Constants();

    selector ==> n2b.in;
    for (i=0; i<4; i++) {
        n2b.out[i] ==> mux.s[i];
    }
    for (i=0; i<16; i++) {
        cst.out[i] ==> mux.c[i];
    }

    mux.out ==> out;
}

component main = Main();
