pragma circom 2.1.3;

include "helpers/arrays.circom";

template check_are_ascii_digits_test(maxNumDigits) {
    signal input in[maxNumDigits];
    signal input len;
    
    CheckAreASCIIDigits(maxNumDigits)(in, len);
}

component main = check_are_ascii_digits_test(
   2000
);
