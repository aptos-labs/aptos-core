pragma circom 2.0.0;

include "../../circuits/escalarmulfix.circom";
include "../../circuits/bitify.circom";


template Main() {
    signal input e;
    signal output out[2];

    var base[2] = [5299619240641551281634865583518297030282874472190772894086521144482721001553,
                   16950150798460657717958625567821834550301663161624707787222815936182638968203];


    component n2b = Num2Bits(253);
    component escalarMul = EscalarMulFix(253, base);

    var i;

    e ==> n2b.in;

    for  (i=0; i<253; i++) {
        n2b.out[i] ==> escalarMul.e[i];
    }

    escalarMul.out[0] ==> out[0];
    escalarMul.out[1] ==> out[1];
}

component main = Main();

