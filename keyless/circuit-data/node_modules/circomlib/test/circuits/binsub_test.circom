pragma circom 2.0.0;

include "../../circuits/bitify.circom";
include "../../circuits/binsub.circom";

template A() {
    signal input a; //private
    signal input b;
    signal output out;

    var i;

    component n2ba = Num2Bits(16);
    component n2bb = Num2Bits(16);
    component sub = BinSub(16);
    component b2n = Bits2Num(16);

    n2ba.in <== a;
    n2bb.in <== b;

    for (i=0; i<16; i++) {
        sub.in[0][i] <== n2ba.out[i];
        sub.in[1][i] <== n2bb.out[i];
    }

    for (i=0; i<16; i++) {
        b2n.in[i] <== sub.out[i];
    }

    out <== b2n.out;
}

component main = A();
