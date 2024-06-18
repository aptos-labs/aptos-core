pragma circom 2.1.3;

include "helpers/arrays.circom";

template right_array_selector_test(len) {
    signal input index;
    signal input expected_output[len];
    
    signal out[len] <== RightArraySelector(len)(index);
    out === expected_output;
}

component main = right_array_selector_test(
   1
);
