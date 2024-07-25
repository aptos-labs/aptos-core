pragma circom 2.1.3;

include "helpers/arrays.circom";

template array_selector_complex_test(len) {
    signal input start_index;
    signal input end_index;
    signal input expected_output[len];
    
    signal out[len] <== ArraySelectorComplex(len)(start_index, end_index);
    out === expected_output;
}

component main = array_selector_complex_test(
   3
);
