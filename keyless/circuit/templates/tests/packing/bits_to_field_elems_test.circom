
pragma circom 2.1.3;

include "helpers/packing.circom";

template bits_to_field_elems_test() {
    var max_bits_len = 256;
    var bits_per_field_elem = 64;
    var num_field_elems = max_bits_len%bits_per_field_elem == 0 ? max_bits_len \ bits_per_field_elem : (max_bits_len\bits_per_field_elem) + 1; // '\' is the quotient operation - we add 1 if there are extra bits past the full bytes
    signal input bits_in[max_bits_len];
    signal input field_elems_out[num_field_elems];
    component bits_to_field_elems = BitsToFieldElems(max_bits_len, bits_per_field_elem);
    bits_to_field_elems.in <== bits_in;
    bits_to_field_elems.elems === field_elems_out;

}

component main = bits_to_field_elems_test();
