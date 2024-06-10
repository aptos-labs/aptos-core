
pragma circom 2.1.3;

include "helpers/packing.circom";

template num2bits_be_test() {
    var max_bits_len = 8;
    signal input num_in;
    signal input bits_out[max_bits_len];
    component num2bits_be = Num2BitsBE(max_bits_len);
    num2bits_be.in <== num_in;
    num2bits_be.out === bits_out;

}

component main = num2bits_be_test();
