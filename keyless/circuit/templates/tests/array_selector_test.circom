pragma circom 2.1.3;

include "helpers/arrays.circom";

template array_selector_test() {
    var len = 8;

    signal input start_index;
    signal input end_index;
    signal input expected_out[len];

    component array_selector = ArraySelector(len);

    array_selector.start_index <== start_index;
    array_selector.end_index <== end_index;

    array_selector.out === expected_out;

}

component main = array_selector_test();
