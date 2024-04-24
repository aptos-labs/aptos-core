pragma circom 2.1.3;

include "helpers/misc.circom";
include "helpers/arrays.circom";
include "helpers/hashtofield.circom";
include "helpers/packing.circom";
include "../node_modules/circomlib/circuits/gates.circom";
include "../node_modules/circomlib/circuits/bitify.circom";

// Checks the given jwt key value pair has a colon in between the name and value, a comma or endbrace at the end, and only whitespace in between the name and colon, colon and value, and value and end character. Returns the name and value fields 
// We did this instead of a polynomial concatenation check to avoid having to implement a multi-variable concatenation check
template ParseJWTFieldWithQuotedValue(maxKVPairLen, maxNameLen, maxValueLen) {
    signal input field[maxKVPairLen]; // ASCII
    signal input name[maxNameLen];
    signal input value[maxValueLen];
    signal input field_len; // ASCII
    signal input name_len;
    signal input value_index; // index of value within `field`
    signal input value_len;
    signal input colon_index; // index of colon within `field`

    // Enforce that end of name < colon < start of value and that field_len >=
    // name_len + value_len + 1 (where the +1 is for the colon), so that the
    // parts of the JWT field are in the correct order
    signal colon_greater_name <== LessThan(20)([name_len, colon_index]);
    colon_greater_name === 1;
    signal colon_less_value <== LessThan(20)([colon_index, value_index]);
    colon_less_value === 1;
    signal field_len_ok <== GreaterEqThan(20)([field_len, name_len + value_len + 1]);
    field_len_ok === 1;

    signal field_hash <== HashBytesToFieldWithLen(maxKVPairLen)(field, field_len);

    signal name_first_quote <== SelectArrayValue(maxKVPairLen)(field, 0);
    name_first_quote === 34; // '"'
    CheckSubstrInclusionPoly(maxKVPairLen, maxNameLen)(field, field_hash, name, name_len, 1);
    signal name_second_quote <== SelectArrayValue(maxKVPairLen)(field, name_len+1);
    name_second_quote === 34; // '"'

    signal colon <== SelectArrayValue(maxKVPairLen)(field, colon_index);
    colon === 58; // ':'

    signal value_first_quote <== SelectArrayValue(maxKVPairLen)(field, value_index-1);
    value_first_quote === 34; // '"'
    CheckSubstrInclusionPoly(maxKVPairLen, maxValueLen)(field, field_hash, value, value_len, value_index);
    signal value_second_quote <== SelectArrayValue(maxKVPairLen)(field, value_index+value_len);
    value_second_quote === 34; // '"'

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
        log(i, ": ", whitespace_selector_two[i]);
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
    }
}

template ParseJWTFieldWithUnquotedValue(maxKVPairLen, maxNameLen, maxValueLen) {
    signal input field[maxKVPairLen]; // ASCII
    signal input name[maxNameLen];
    signal input value[maxValueLen];
    signal input field_len; // ASCII
    signal input name_len;
    signal input value_index; // index of value within `field`
    signal input value_len;
    signal input colon_index; // index of colon within `field`

    // Enforce that end of name < colon < start of value and that field_len >=
    // name_len + value_len + 1 (where the +1 is for the colon), so that the
    // parts of the JWT field are in the correct order
    signal colon_greater_name <== LessThan(20)([name_len, colon_index]);
    colon_greater_name === 1;
    signal colon_less_value <== LessThan(20)([colon_index, value_index]);
    colon_less_value === 1;
    signal field_len_ok <== GreaterEqThan(20)([field_len, name_len + value_len + 1]);
    field_len_ok === 1;


    signal field_hash <== HashBytesToFieldWithLen(maxKVPairLen)(field, field_len);

    signal name_first_quote <== SelectArrayValue(maxKVPairLen)(field, 0);
    name_first_quote === 34; // '"'
    CheckSubstrInclusionPoly(maxKVPairLen, maxNameLen)(field, field_hash, name, name_len, 1);
    signal name_second_quote <== SelectArrayValue(maxKVPairLen)(field, name_len+1);
    name_second_quote === 34; // '"'

    signal colon <== SelectArrayValue(maxKVPairLen)(field, colon_index);
    colon === 58; // ':'

    // Don't check for quotes around values, since this is the unquoted variant
    CheckSubstrInclusionPoly(maxKVPairLen, maxValueLen)(field, field_hash, value, value_len, value_index);

    // Enforce last character of `field` is comma or end brace
    signal last_char <== SelectArrayValue(maxKVPairLen)(field, field_len-1);
    (last_char - 44) * (last_char - 125) === 0; // ',' or '}'

    // Verify whitespace is in right places
    signal is_whitespace[maxKVPairLen];
    for (var i = 0; i < maxKVPairLen; i++) {
        is_whitespace[i] <== isWhitespace()(field[i]);
    }

    signal whitespace_selector_one[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(name_len+2, colon_index); // Skip 2 quotes around name, stop 1 index before the colon
    signal whitespace_selector_two[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(colon_index+1, value_index); 
    signal whitespace_selector_three[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(value_index+value_len, field_len-1); 

    for (var i = 0; i < maxKVPairLen; i++) {
        log(i, ": ", whitespace_selector_two[i]);
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
    }
}
