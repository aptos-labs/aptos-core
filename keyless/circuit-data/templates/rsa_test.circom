pragma circom 2.1.3;

include "helpers/base64.circom";
include "helpers/slice.circom";
include "helpers/arrays.circom";
include "helpers/misc.circom";
include "helpers/packing.circom";
include "helpers/hashtofield.circom";
include "helpers/sha.circom";
include "../node_modules/circomlib/circuits/sha256/sha256.circom";
//include "../doubleblind-xyz/circuits/rsa.circom";
include "../circom-rsa-verify/circuits/rsa_verify.circom";
include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/bitify.circom";


template test_rsa() {

    signal input sig[32];
    signal input mod[32];
    signal input hash[4];

    RsaVerifyPkcs1v15(64, 32)(sig, mod, hash);

//    signal input a[2];
//    signal input b[2];
//    signal input mod[2];
//
//    signal out[2] <== FpMul(64, 2)(a, b, mod);
//    log(out[0]);
//    log(out[1]);

}

component main = test_rsa(); 
