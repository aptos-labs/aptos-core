pragma circom 2.1.3;

include "../node_modules/circomlib/circuits/bitify.circom";
include "../node_modules/circomlib/circuits/gates.circom";

// Checks the given jwt key value pair has a colon in between the name and value, a comma or endbrace at the end, and only whitespace in between the name and colon, colon and value, and value and end character. Returns the name and value fields 
// We did this instead of a polynomial concatenation check to avoid having to implement a multi-variable concatenation check
template JWTFieldCheck(maxKVPairLen, maxNameLen, maxValueLen) {
    signal input field[maxKVPairLen]; // ASCII
    signal input field_len; // ASCII
    signal input index; // index of field in ASCII jwt
    signal input name_len;
    signal input value_index; // index of value within `field`
    signal input value_len;
    signal input colon_index; // index of colon within `field`
    signal input name[maxNameLen];
    signal input value[maxValueLen];

    // Enforce that end of name < colon < start of value, so that these 3 parts of the JWT field are in the correct order
    signal colon_greater_name <== LessThan(20)([name_len, colon_index]);
    colon_greater_name === 1;
    signal colon_less_value <== LessThan(20)([colon_index, value_index]);
    colon_less_value === 1;

    signal field_hash <== HashBytesToFieldWithLen(maxKVPairLen)(field, field_len);

    signal first_quote <== SelectArrayValue(maxKVPairLen)(field, 0);
    first_quote === 34; // '"'
    CheckSubstrInclusionPoly(maxKVPairLen, maxNameLen)(field, field_hash, name, name_len, 1);
    signal second_quote <== SelectArrayValue(maxKVPairLen)(field, name_len+1);
    second_quote === 34; // '"'

    signal colon <== SelectArrayValue(maxKVPairLen)(field, colon_index);
    colon === 58; // ':'

    // TODO: Do we need to check quotes around values? Should we? Some don't have quotes so probably not
    //signal third_quote <== SelectArrayValue(maxKVPairLen)(field, value_index-1);
    //third_quote === 34; // '"'
    CheckSubstrInclusionPoly(maxKVPairLen, maxValueLen)(field, field_hash, value, value_len, value_index);
    //signal fourth_quote <== SelectArrayValue(maxKVPairLen)(field, value_index+value_len);
    //fourth_quote === 34; // '"'

    // Enforce last character of `field` is comma or end brace
    signal last_char <== SelectArrayValue(maxKVPairLen)(field, field_len-1);
    (last_char - 44) * (last_char - 125) === 0; // ',' or '}'

    // Verify whitespace is in right places
    signal is_whitespace[maxKVPairLen];
    for (var i = 0; i < maxKVPairLen; i++) {
        is_whitespace[i] <== isWhitespace()(field[i]);
    }

    signal whitespace_selector_one[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(name_len+2, colon_index); // Skip 2 quotes around name, stop 1 index before the colon
    signal whitespace_selector_two[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(colon_index+1, value_index-1); // Skip 2 quotes around value, stop 1 index before the value
    signal whitespace_selector_three[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(value_index+value_len+1, field_len-1); // Skip 2 quotes in the value, stop just before the comma/end brace

    for (var i = 0; i < maxKVPairLen; i++) {
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
    }
}

// Checks if character 'char' is a whitespace character. Returns 1 if so, 0 otherwise
template isWhitespace() {
   signal input char;  
                       
   signal is_tab <== IsEqual()([char, 9]); // character is a tab space
   signal is_newline <== IsEqual()([char, 10]); // character is a newline 
   signal is_carriage_return <== IsEqual()([char, 13]); // character is a carriage return
   signal is_space <== IsEqual()([char, 32]); // ' '
                       
   signal output is_whitespace <== is_tab + is_newline + is_carriage_return + is_space;
}

// https://github.com/TheFrozenFire/snark-jwt-verify/blob/master/circuits/calculate_total.circom
// This circuit returns the sum of the inputs.
// n must be greater than 0.
template CalculateTotal(n) {
    signal input nums[n];
    signal output sum;

    signal sums[n];
    sums[0] <== nums[0];

    for (var i=1; i < n; i++) {
        sums[i] <== sums[i - 1] + nums[i];
    }

    sum <== sums[n - 1];
}

// Checks `input_not_ascii` is the digit representation of ascii digit string `input_ascii`
// Assumes both inputs contain only digits
template DigitStringAsciiEquivalenceCheck(maxInputLen) {
    signal input input_ascii[maxInputLen];
    signal input input_not_ascii[maxInputLen];
    signal input len;

    signal selector_bits[maxInputLen] <== RightArraySelector(maxInputLen)(len-1);
    for (var i = 0; i < maxInputLen; i++) {
        ((input_ascii[i]-48)-input_not_ascii[i])*(1-selector_bits[i]) === 0;
    }
}

// Given input `in`, enforces that `in[0] === in[1]` if `bool` is 1
template AssertEqualIfTrue() {
    signal input in[2];
    signal input bool;

    (in[0]-in[1]) * bool === 0;
}

// Enforce that if uid name is "email", the email verified field is either true or "true"
template EmailVerifiedCheck(maxEVNameLen, maxEVValueLen, maxUIDNameLen) {
    signal input ev_name[maxEVNameLen];
    signal input ev_value[maxEVValueLen];
    signal input ev_value_len;
    signal input uid_name[maxUIDNameLen];
    signal input uid_name_len;
    signal output uid_is_email;

    var email[5] = [101, 109, 97, 105, 108]; // email

    var uid_starts_with_email_0 = IsEqual()([email[0], uid_name[0]]);
    var uid_starts_with_email_1 = IsEqual()([email[1], uid_name[1]]);
    var uid_starts_with_email_2 = IsEqual()([email[2], uid_name[2]]);
    var uid_starts_with_email_3 = IsEqual()([email[3], uid_name[3]]);
    var uid_starts_with_email_4 = IsEqual()([email[4], uid_name[4]]);
    var uid_starts_with_email = MultiAND(5)([uid_starts_with_email_0, uid_starts_with_email_1, uid_starts_with_email_2, uid_starts_with_email_3, uid_starts_with_email_4]);


    signal uid_name_len_is_5 <== IsEqual()([uid_name_len, 5]);
    uid_is_email <== AND()(uid_starts_with_email, uid_name_len_is_5); // '1' if uid_name is "email" with length 5. This guarantees uid_name is in fact "email" (with quotes) combined with the logic in `JWTFieldCheck`

    var required_ev_name[14] = [101, 109, 97, 105, 108, 95, 118, 101, 114, 105, 102, 105, 101, 100];    // email_verified

    // If uid name is "email", enforce ev_name is "email_verified"
    for (var i = 0; i < 14; i++) {
        AssertEqualIfTrue()([ev_name[i], required_ev_name[i]], uid_is_email);
    }

    signal ev_val_len_is_4 <== IsEqual()([ev_value_len, 4]);
    signal ev_val_len_is_6 <== IsEqual()([ev_value_len, 6]);
    var ev_val_len_is_correct = OR()(ev_val_len_is_4, ev_val_len_is_6);

    signal not_uid_is_email <== NOT()(uid_is_email);
    signal is_ok <== OR()(not_uid_is_email, ev_val_len_is_correct);
    is_ok === 1;
    
    var required_ev_val_len_4[4] = [116, 114, 117, 101]; // true
    signal check_ev_val_bool <== AND()(ev_val_len_is_4, uid_is_email);
    for (var i = 0; i < 4; i ++) {
        AssertEqualIfTrue()([required_ev_val_len_4[i], ev_value[i]], check_ev_val_bool);
    }

    var required_ev_val_len_6[6] = [34, 116, 114, 117, 101, 34]; // "true"
    signal check_ev_val_str <== AND()(ev_val_len_is_6, uid_is_email);
    for (var i = 0; i < 6; i++) {
        AssertEqualIfTrue()([required_ev_val_len_6[i], ev_value[i]], check_ev_val_str);
    }
}

// Given a base64-encoded array `in`, max length `maxN`, and actual unpadded length `n`, returns
// the actual length of the decoded string
template Base64DecodedLength(maxN) {
    var max_q = (3 * maxN) \ 4;
    //signal input in[maxN];
    signal input n; // actual lenght
    signal output decoded_len;
    signal q <-- 3*n \ 4;
    signal r <-- 3*n % 4;

    3*n - 4*q - r === 0;
    signal r_correct_reminder <== LessThan(2)([r, 4]);
    r_correct_reminder === 1;

    // use log function to compute log(max_q)
    signal q_correct_quotient <== LessThan(252)([q, max_q]);
    q_correct_quotient === 1;

    // var eq = 61;
    // assumes valid encoding (if last != "=" then second to last is also
    // != "=")
    // TODO: We don't seem to need this, as the jwt spec removes b64 padding
    // see https://datatracker.ietf.org/doc/html/rfc7515#page-54
    //signal l <== SelectArrayValue(maxN)(in, n - 1);
    //signal s2l <== SelectArrayValue(maxN)(in, n - 2);
    //signal s_l <== IsEqual()([l, eq]);
    //signal s_s2l <== IsEqual()([s2l, eq]);
    //signal reducer <== -1*s_l -1*s_s2l;
    //decoded_len <== q + reducer;
    //log("decoded_len", decoded_len);
}
