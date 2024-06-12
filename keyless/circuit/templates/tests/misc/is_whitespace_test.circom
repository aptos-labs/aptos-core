
pragma circom 2.1.3;

include "helpers/misc.circom";

template is_whitespace_test() {
    signal input char;
    signal input result;
    component is_whitespace = isWhitespace();
    is_whitespace.char <== char;
    is_whitespace.is_whitespace === result;

}

component main = is_whitespace_test(
);
