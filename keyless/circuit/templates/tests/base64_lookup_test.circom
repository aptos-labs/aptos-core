
pragma circom 2.1.3;

include "helpers/base64.circom";

template base64_lookup_test() {
    signal input in_b64_char;
    signal input out_num;
    component base64_url_lookup = Base64URLLookup();
    base64_url_lookup.in <== in_b64_char;
    out_num === base64_url_lookup.out;

}

component main = base64_lookup_test();
