pragma circom 2.1.3;

// File taken from https://github.com/zkemail/zk-email-verify/blob/main/packages/circuits/helpers/base64.circom

include "circomlib/circuits/comparators.circom";

// http://0x80.pl/notesen/2016-01-17-sse-base64-decoding.html#vector-lookup-base
// Modified to support Base64URL format instead of Base64
// Also accepts zero padding, which is not in the Base64/Base64URL format
template Base64URLLookup() {
    signal input in;
    signal output out;

    // ['A', 'Z']
    component le_Z = LessThan(8);
    le_Z.in[0] <== in;
    le_Z.in[1] <== 90+1;

    component ge_A = GreaterThan(8);
    ge_A.in[0] <== in;
    ge_A.in[1] <== 65-1;

    signal range_AZ <== ge_A.out * le_Z.out;
    signal sum_AZ <== range_AZ * (in - 65);

    // ['a', 'z']
    component le_z = LessThan(8);
    le_z.in[0] <== in;
    le_z.in[1] <== 122+1;

    component ge_a = GreaterThan(8);
    ge_a.in[0] <== in;
    ge_a.in[1] <== 97-1;

    signal range_az <== ge_a.out * le_z.out;
    signal sum_az <== sum_AZ + range_az * (in - 71);

    // ['0', '9']
    component le_9 = LessThan(8);
    le_9.in[0] <== in;
    le_9.in[1] <== 57+1;

    component ge_0 = GreaterThan(8);
    ge_0.in[0] <== in;
    ge_0.in[1] <== 48-1;

    signal range_09 <== ge_0.out * le_9.out;
    signal sum_09 <== sum_az + range_09 * (in + 4);

    // '-'
    component equal_minus = IsZero();
    equal_minus.in <== in - 45;
    // https://www.cs.cmu.edu/~pattis/15-1XX/common/handouts/ascii.html ascii '-' (45)
    // https://base64.guru/learn/base64-characters  == 62 in base64
    signal sum_minus <== sum_09 + equal_minus.out * 62;

    // '_'
    component equal_underscore = IsZero();
    equal_underscore.in <== in - 95;
    // https://www.cs.cmu.edu/~pattis/15-1XX/common/handouts/ascii.html ascii '_' (95)
    // https://base64.guru/learn/base64-characters == 63 in base64
    signal sum_underscore <== sum_minus + equal_underscore.out * 63;

    out <== sum_underscore;
    log("sum_underscore (out): ", out);

    // '='
    component equal_eqsign = IsZero();
    equal_eqsign.in <== in - 61;

    // Also decode zero padding as zero padding
    component zero_padding = IsZero();
    zero_padding.in <== in;


    log("zero_padding.out: ", zero_padding.out);
    log("equal_eqsign.out: ", equal_eqsign.out);
    log("equal_underscore.out: ", equal_underscore.out);
    log("equal_minus.out: ", equal_minus.out);
    log("range_09: ", range_09);
    log("range_az: ", range_az);
    log("range_AZ: ", range_AZ);
    log("< end Base64URLLookup");

    signal result <== range_AZ + range_az + range_09 + equal_minus.out + equal_underscore.out + equal_eqsign.out + zero_padding.out;
    1 === result;
}

template Base64Decode(N) {
    //var N = ((3*M)\4)+2; // TODO: Make sure this is ok
    var M = 4*((N+2)\3);
    signal input in[M];
    signal output out[N];

    component bits_in[M\4][4];
    component bits_out[M\4][3];
    component translate[M\4][4];

    var idx = 0;
    for (var i = 0; i < M; i += 4) {
        for (var j = 0; j < 3; j++) {
            bits_out[i\4][j] = Bits2Num(8);
        }


        //     log("range_AZ: ", range_AZ);
        for (var j = 0; j < 4; j++) {
            bits_in[i\4][j] = Num2Bits(6);

            log(">> calling into Base64URLLookup");
            log("translate[i\\4][j].in: ", in[i+j]);

            translate[i\4][j] = Base64URLLookup();
            translate[i\4][j].in <== in[i+j];
            translate[i\4][j].out ==> bits_in[i\4][j].in;
        }

        // Do the re-packing from four 6-bit words to three 8-bit words.
        for (var j = 0; j < 6; j++) {
            bits_out[i\4][0].in[j+2] <== bits_in[i\4][0].out[j];
        }
        bits_out[i\4][0].in[0] <== bits_in[i\4][1].out[4];
        bits_out[i\4][0].in[1] <== bits_in[i\4][1].out[5];

        for (var j = 0; j < 4; j++) {
            bits_out[i\4][1].in[j+4] <== bits_in[i\4][1].out[j];
        }
        for (var j = 0; j < 4; j++) {
            bits_out[i\4][1].in[j] <== bits_in[i\4][2].out[j+2];
        }

        bits_out[i\4][2].in[6] <== bits_in[i\4][2].out[0];
        bits_out[i\4][2].in[7] <== bits_in[i\4][2].out[1];
        for (var j = 0; j < 6; j++) {
            bits_out[i\4][2].in[j] <== bits_in[i\4][3].out[j];
        }

        for (var j = 0; j < 3; j++) {
            if (idx+j < N) {
                out[idx+j] <== bits_out[i\4][j].out;
            }
        }
        idx += 3;
    }
}
