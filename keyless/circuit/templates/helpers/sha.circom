pragma circom 2.1.3;

include "circomlib/circuits/sha256/constants.circom";
include "circomlib/circuits/sha256/sha256compression.circom";
include "circomlib/circuits/comparators.circom";
include "./misc.circom";

// Similar to `sha256_unsafe` in https://github.com/TheFrozenFire/snark-jwt-verify/blob/master/circuits/sha256.circom
// Hashes a bit array message using SHA2_256, hashing every block up to and including `tBlock`. All blocks after `tBlock` are ignored in the output
// Expects the bit array input to be padded according to https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1 up to tBlock. 
template Sha2_256_prepadded_varlen(maxNumBlocks) {
    signal input in[maxNumBlocks * 512];
    signal input tBlock;
    
    signal output out[256];

    component ha0 = H(0);
    component hb0 = H(1);
    component hc0 = H(2);
    component hd0 = H(3);
    component he0 = H(4);
    component hf0 = H(5);
    component hg0 = H(6);
    component hh0 = H(7);

    component sha256compression[maxNumBlocks];

    for(var i=0; i < maxNumBlocks; i++) {

        sha256compression[i] = Sha256compression();

        if (i==0) {
            for(var k = 0; k < 32; k++) {
                sha256compression[i].hin[0*32+k] <== ha0.out[k];
                sha256compression[i].hin[1*32+k] <== hb0.out[k];
                sha256compression[i].hin[2*32+k] <== hc0.out[k];
                sha256compression[i].hin[3*32+k] <== hd0.out[k];
                sha256compression[i].hin[4*32+k] <== he0.out[k];
                sha256compression[i].hin[5*32+k] <== hf0.out[k];
                sha256compression[i].hin[6*32+k] <== hg0.out[k];
                sha256compression[i].hin[7*32+k] <== hh0.out[k];
            }
        } else {
            for(var k = 0; k < 32; k++) {
                sha256compression[i].hin[32*0+k] <== sha256compression[i-1].out[32*0+31-k];
                sha256compression[i].hin[32*1+k] <== sha256compression[i-1].out[32*1+31-k];
                sha256compression[i].hin[32*2+k] <== sha256compression[i-1].out[32*2+31-k];
                sha256compression[i].hin[32*3+k] <== sha256compression[i-1].out[32*3+31-k];
                sha256compression[i].hin[32*4+k] <== sha256compression[i-1].out[32*4+31-k];
                sha256compression[i].hin[32*5+k] <== sha256compression[i-1].out[32*5+31-k];
                sha256compression[i].hin[32*6+k] <== sha256compression[i-1].out[32*6+31-k];
                sha256compression[i].hin[32*7+k] <== sha256compression[i-1].out[32*7+31-k];
            }
        }

        for (var k = 0; k < 512; k++) {
            sha256compression[i].inp[k] <== in[i*512 + k];
        }
    }
    
    // Collapse the hashing result at the terminating data block
    component calcTotal[256];
    signal eqs[maxNumBlocks] <== SingleOneArray(maxNumBlocks)(tBlock);

    // For each bit of the output
    for(var k = 0; k < 256; k++) {
        calcTotal[k] = CalculateTotal(maxNumBlocks);
        
        // For each possible block
        for (var i = 0; i < maxNumBlocks; i++) {

            // eqs[i] is 1 if the index matches. As such, at most one input to calcTotal is not 0.
            // The bit corresponding to the terminating data block will be raised
            calcTotal[k].nums[i] <== eqs[i] * sha256compression[i].out[k];
        }
        
        out[k] <== calcTotal[k].sum;
    }
}

// Verifies SHA2_256 input padding according to https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1
template Sha2PaddingVerify(maxInputLen) {
    signal input in[maxInputLen]; // byte array
    signal input num_blocks; // Number of 512-bit blocks in `in` including sha padding
    signal input padding_start; // equivalent to L/8, where L is the length of the unpadded message in bits as specified in RFC4634
    signal input L_byte_encoded[8]; // 64-bit encoding of L
    signal input padding_without_len[64]; // padding_without_len[0] = 1, followed by K 0s. Length K+1, max length 512 bits. Does not include the 64-bit encoding of L

    var len_bits = num_blocks * 512;
    var padding_start_bits = padding_start * 8;
    var K = len_bits - padding_start_bits - 1 - 64; 

    // Ensure K is smallest value below 512 that satisfies above equation
    signal K_is_correct <== LessThan(10)([K,512]);
    K_is_correct === 1;

    signal in_hash <== HashBytesToFieldWithLen(maxInputLen)(in, num_blocks*64);
    // 4.1.a
    CheckSubstrInclusionPoly(maxInputLen, 64)(in, in_hash, padding_without_len, (1+K)/8, padding_start);
    padding_without_len[0] === 128; // 10000000

    // 4.1.b
    for (var i = 1; i < 64; i++) {
        padding_without_len[i] === 0;
    }

    // 4.1.c
    CheckSubstrInclusionPoly(maxInputLen, 8)(in, in_hash, L_byte_encoded, 8, padding_start+(K+1)/8);
    signal L_bits[64] <== BytesToBits(8)(L_byte_encoded);
    signal L_decoded <== Bits2NumBigEndian(64)(L_bits);
    L_decoded === 8*padding_start;
}
