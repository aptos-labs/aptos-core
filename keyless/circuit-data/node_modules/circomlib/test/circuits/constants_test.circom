pragma circom 2.0.0;

include "../../circuits/sha256/constants.circom";

template A() {
    signal input in;
    component h0;
    h0 = K(8);

    var lc = 0;
    var e = 1;
    for (var i=0; i<32; i++) {
        lc = lc + e*h0.out[i];
        e *= 2;
    }

    lc === in;
}

component main {public [in]} = A();
