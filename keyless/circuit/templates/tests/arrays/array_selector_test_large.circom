pragma circom 2.1.3;

include "helpers/arrays.circom";

template array_selector_test(len) {
    signal input start_index;
    signal input end_index;
    signal input expected_output[len];
    
    signal out[len] <== ArraySelector(len)(start_index, end_index);
    out === expected_output;
}

component main = array_selector_test(
   2000
);
