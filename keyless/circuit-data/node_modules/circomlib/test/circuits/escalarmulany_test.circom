pragma circom 2.0.0;

include "../../circuits/escalarmulany.circom";
include "../../circuits/bitify.circom";

template Main() {
    signal input e;
    signal input p[2];
    signal output out[2];

    component n2b = Num2Bits(253);
    component escalarMulAny = EscalarMulAny(253);

    escalarMulAny.p[0] <== p[0];
    escalarMulAny.p[1] <== p[1];

    var i;

    e ==> n2b.in;

    for  (i=0; i<253; i++) {
        n2b.out[i] ==> escalarMulAny.e[i];
    }

    escalarMulAny.out[0] ==> out[0];
    escalarMulAny.out[1] ==> out[1];
}

component main = Main();

