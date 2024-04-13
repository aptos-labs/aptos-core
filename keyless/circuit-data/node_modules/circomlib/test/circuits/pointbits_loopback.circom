pragma circom 2.0.0;

include "../../circuits/pointbits.circom";


template Main() {
    signal input in[2];

    var i;

    component p2b = Point2Bits_Strict();
    component b2p = Bits2Point_Strict();

    p2b.in[0] <== in[0];
    p2b.in[1] <== in[1];

    for (i=0; i<256; i++) {
        b2p.in[i] <== p2b.out[i];
    }

    b2p.out[0] === in[0];
    b2p.out[1] === in[1];
}

component main = Main();
