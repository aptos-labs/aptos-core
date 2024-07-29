pragma circom 2.1.3;

include "helpers/arrays.circom";

template left_array_selector_test(len) {
    signal input index;
    signal input expected_output[len];
    
    signal out[len] <== LeftArraySelector(len)(index);
    out === expected_output;
}

component main = left_array_selector_test(
   8
);
