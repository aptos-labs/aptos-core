

pragma circom 2.1.3;

include "helpers/arrays.circom";

template check_substr_inclusion_poly_boolean_test() {
    var max_str_len = 256;
    var max_substr_len = 8;

    signal input str[max_str_len];
    signal input str_hash;
    signal input substr[max_substr_len];
    signal input substr_len;
    signal input start_index;
    signal input check_passes;

    component check_substr_inclusion_poly_boolean = CheckSubstrInclusionPolyBoolean(max_str_len, max_substr_len);

    check_substr_inclusion_poly_boolean.str <== str;
    check_substr_inclusion_poly_boolean.str_hash <== str_hash;
    check_substr_inclusion_poly_boolean.substr <== substr;
    check_substr_inclusion_poly_boolean.substr_len <== substr_len;
    check_substr_inclusion_poly_boolean.start_index <== start_index;
    check_substr_inclusion_poly_boolean.check_passes === check_passes;
}

component main = check_substr_inclusion_poly_boolean_test();
