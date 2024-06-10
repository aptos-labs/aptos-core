pragma circom 2.1.3;

include "./misc.circom";
include "./arrays.circom";
include "./hashtofield.circom";
include "./packing.circom";
include "circomlib/circuits/gates.circom";
include "circomlib/circuits/bitify.circom";

template ParseJWTFieldSharedLogic(maxKVPairLen, maxNameLen, maxValueLen) {
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

    CheckSubstrInclusionPoly(maxKVPairLen, maxValueLen)(field, field_hash, value, value_len, value_index);

    // Enforce last character of `field` is comma or end brace
    signal last_char <== SelectArrayValue(maxKVPairLen)(field, field_len-1);
    (last_char - 44) * (last_char - 125) === 0; // ',' or '}'
}

// Checks the given jwt key value pair has a colon in between the name and value, a comma or endbrace at the end, and only whitespace in between the name and colon, colon and value, and value and end character. Returns the name and value fields 
// We did this instead of a polynomial concatenation check to avoid having to implement a multi-variable concatenation check
template ParseJWTFieldWithQuotedValue(maxKVPairLen, maxNameLen, maxValueLen) {
    signal input field[maxKVPairLen]; // ASCII
    signal input name[maxNameLen];
    signal input value[maxValueLen];
    signal input field_string_bodies[maxKVPairLen];
    signal input field_len; // ASCII
    signal input name_len;
    signal input value_index; // index of value within `field`
    signal input value_len;
    signal input colon_index; // index of colon within `field`

    ParseJWTFieldSharedLogic(maxKVPairLen, maxNameLen, maxValueLen)(field, name, value, field_len, name_len, value_index, value_len, colon_index);

    signal value_first_quote <== SelectArrayValue(maxKVPairLen)(field, value_index-1);
    value_first_quote === 34; // '"'
    signal value_second_quote <== SelectArrayValue(maxKVPairLen)(field, value_index+value_len);
    value_second_quote === 34; // '"'


    // Verify whitespace is in right places, and that only name and value are inside string bodies
    signal is_whitespace[maxKVPairLen];
    for (var i = 0; i < maxKVPairLen; i++) {
        is_whitespace[i] <== isWhitespace()(field[i]);
    }

    signal whitespace_selector_one[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(name_len+2, colon_index); // Skip 2 quotes around name, stop 1 index before the colon
    signal whitespace_selector_two[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(colon_index+1, value_index-1); // Skip 2 quotes around value, stop 1 index before the value
    signal whitespace_selector_three[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(value_index+value_len+1, field_len-1); // Skip 2 quotes in the value, stop just before the comma/end brace
    signal name_selector[maxKVPairLen] <== ArraySelector(maxKVPairLen)(1, name_len+1);
    signal value_selector[maxKVPairLen] <== ArraySelector(maxKVPairLen)(value_index, value_index+value_len);


    for (var i = 0; i < maxKVPairLen; i++) {
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
        
        // Check that only the name and value parts of the field are inside string bodies, and nothing else is
        (name_selector[i] + value_selector[i]) * (1 - field_string_bodies[i]) === 0;
        (1 - (name_selector[i] + value_selector[i])) * field_string_bodies[i] === 0;
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

    ParseJWTFieldSharedLogic(maxKVPairLen, maxNameLen, maxValueLen)(field, name, value, field_len, name_len, value_index, value_len, colon_index);

    // Verify whitespace is in right places
    signal is_whitespace[maxKVPairLen];
    for (var i = 0; i < maxKVPairLen; i++) {
        is_whitespace[i] <== isWhitespace()(field[i]);
    }

    signal whitespace_selector_one[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(name_len+2, colon_index); // Skip 2 quotes around name, stop 1 index before the colon
    signal whitespace_selector_two[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(colon_index+1, value_index); // no quote this time, so check whitespace until the value start
    signal whitespace_selector_three[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(value_index+value_len, field_len-1); // and directly after the value end

    for (var i = 0; i < maxKVPairLen; i++) {
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
    }

    // Verify value does not contain comma, end brace, or quote
    signal value_selector[maxKVPairLen] <== ArraySelector(maxKVPairLen)(value_index, value_index+value_len);

    for (var i = 0; i < maxKVPairLen; i++) {
        var is_comma = IsEqual()([field[i], 44]);
        var is_end_brace = IsEqual()([field[i], 125]);
        var is_quote = IsEqual()([field[i], 34]);
        value_selector[i] * (is_comma + is_end_brace + is_quote) === 0;
    }
}

template ParseEmailVerifiedField(maxKVPairLen, maxNameLen, maxValueLen) {
    signal input field[maxKVPairLen]; // ASCII
    signal input name[maxNameLen];
    signal input value[maxValueLen];
    signal input field_len; // ASCII
    signal input name_len;
    signal input value_index; // index of value within `field`
    signal input value_len;
    signal input colon_index; // index of colon within `field`


    ParseJWTFieldSharedLogic(maxKVPairLen, maxNameLen, maxValueLen)(field, name, value, field_len, name_len, value_index, value_len, colon_index);


    signal char_before_value <== SelectArrayValue(maxKVPairLen)(field, value_index-1);
    signal before_is_quote      <== IsEqual()([char_before_value, 34]);
    signal before_is_whitespace <== isWhitespace()(char_before_value);
    signal before_is_whitespace_or_quote <== OR()(before_is_quote, before_is_whitespace);
    // Check the char before is either quote or whitespace, OR that it is the colon
    (1 - before_is_whitespace_or_quote)*(value_index-1-colon_index) === 0;
    signal char_after_value <== SelectArrayValue(maxKVPairLen)(field, value_index+value_len);
    signal after_is_quote       <== IsEqual()([char_after_value, 34]);
    signal after_is_whitespace  <== isWhitespace()(char_after_value);
    // check OR(after_is_quote, after_is_whitespace) === 1.
    signal after_is_whitespace_or_quote <== OR()(after_is_quote, after_is_whitespace);
    // Check the char after is either quote or whitespace, OR that it is the field delimiter
    (1 - after_is_whitespace_or_quote)*(field_len-1-value_index-value_len) === 0;

    // Check that field value doesn't have mismatched quotes
    signal and_1 <== AND()(before_is_quote,after_is_whitespace);
    signal and_2 <== AND()(before_is_whitespace,after_is_quote);
    and_1 + and_2 === 0;


    signal is_whitespace[maxKVPairLen];
    for (var i = 0; i < maxKVPairLen; i++) {
        is_whitespace[i] <== isWhitespace()(field[i]);
    }

    signal whitespace_selector_one[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(name_len+2, colon_index); // Skip 2 quotes around name, stop 1 index before the colon
    signal whitespace_selector_two[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(colon_index+1, value_index-1); // There could potentially be quotes around the value, so we don't contstrain the character before value_index to be whitespace
    signal whitespace_selector_three[maxKVPairLen] <== ArraySelectorComplex(maxKVPairLen)(value_index+value_len+1, field_len-1); // similarly to before, don't constrain character just after value end


    signal name_selector[maxKVPairLen] <== ArraySelector(maxKVPairLen)(1, name_len+1);
    signal value_selector[maxKVPairLen] <== ArraySelector(maxKVPairLen)(value_index, value_index+value_len);


    for (var i = 0; i < maxKVPairLen; i++) {
        (whitespace_selector_one[i] + whitespace_selector_two[i] + whitespace_selector_three[i]) * (1 - is_whitespace[i]) === 0;
    }
}




