pragma circom 2.1.3;

include "helpers/rsa/rsa_verify.circom";

component main = RsaVerifyPkcs1v15(64, 32);
