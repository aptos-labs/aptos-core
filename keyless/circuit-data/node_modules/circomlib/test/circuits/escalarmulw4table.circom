pragma circom 2.0.0;

include "../../circuits/escalarmulw4table.circom";




template Main() {
    signal output out[16][2];
    var base[2] = [5299619240641551281634865583518297030282874472190772894086521144482721001553,
                16950150798460657717958625567821834550301663161624707787222815936182638968203];

    var escalarMul[16][2] = EscalarMulW4Table(base, 0);
    for (var i=0; i<16; i++) {
        out[i][0] <== escalarMul[i][0];
        out[i][1] <== escalarMul[i][1];
    }
}

component main = Main();
