pragma circom 2.1.3;

include "helpers/sha.circom";

template sha_test(max_num_blocks) {
    signal input padded_input_bits[max_num_blocks * 512];
    signal input input_bit_len;
    signal input expected_digest_bits[256];
    component c1 = Sha2_256_prepadded_varlen(max_num_blocks);
    c1.in <== padded_input_bits;
    c1.tBlock <== (input_bit_len / 512) - 1;
    expected_digest_bits === c1.out;
}

component main = sha_test(4);
