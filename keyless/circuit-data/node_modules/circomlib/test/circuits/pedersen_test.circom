pragma circom 2.0.0;

include "../../circuits/pedersen_old.circom";
include "../../circuits/bitify.circom";


template Main() {
    signal input in[2];
    signal output out[2];

    component pedersen = Pedersen(250*2);

    component n2b[2];
    n2b[0] = Num2Bits(250);
    n2b[1] = Num2Bits(250);

    var i;

    in[0] ==> n2b[0].in;
    in[1] ==> n2b[1].in;

    for  (i=0; i<250; i++) {
        n2b[0].out[i] ==> pedersen.in[i];
        n2b[1].out[i] ==> pedersen.in[250+i];
    }

    pedersen.out[0] ==> out[0];
    pedersen.out[1] ==> out[1];
}

component main = Main();
