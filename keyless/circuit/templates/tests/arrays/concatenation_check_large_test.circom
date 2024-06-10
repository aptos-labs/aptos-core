pragma circom 2.1.3;

include "helpers/arrays.circom";

template concatenation_check_test(maxFullStringLen, maxLeftStringLen, maxRightStringLen) {
    signal input full_string[maxFullStringLen];
    signal input left[maxLeftStringLen];
    signal input right[maxRightStringLen];
    signal input left_len;
    signal input right_len;
    
    log("silly test string");
    ConcatenationCheck(maxFullStringLen, maxLeftStringLen, maxRightStringLen)(full_string, left, right, left_len, right_len);
}

component main = concatenation_check_test(
   1600, 1000, 1000
);
