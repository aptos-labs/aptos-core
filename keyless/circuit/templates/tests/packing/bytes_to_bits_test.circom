pragma circom 2.1.3;

include "helpers/packing.circom";

template bytes_to_bits_test() {
    var max_bytes_len = 10;
    var max_bits_len = max_bytes_len * 8;
    signal input bytes_in[max_bytes_len];
    signal input bits_out[max_bits_len];
    component bytes_to_bits = BytesToBits(max_bytes_len);
    bytes_to_bits.in <== bytes_in;
    bytes_to_bits.bits === bits_out;

}

component main = bytes_to_bits_test();
