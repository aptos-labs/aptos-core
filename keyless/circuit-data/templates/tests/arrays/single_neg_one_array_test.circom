pragma circom 2.1.3;

include "helpers/arrays.circom";

template single_neg_one_array_test(len) {
    signal input index;
    signal input expected_output[len];
    
    signal out[len] <== SingleNegOneArray(len)(index);
    out === expected_output;
}

component main = single_neg_one_array_test(
   8
);
