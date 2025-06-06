module aptos_framework::base16 {

    use std::vector;

    friend aptos_framework::ethereum_derivable_account;

    // Convert a hex character to a u8
    public(friend) fun hex_char_to_u8(c: u8): u8 {
        if (c >= 48 && c <= 57) {  // '0' to '9'
            c - 48
        } else if (c >= 65 && c <= 70) { // 'A' to 'F'
            c - 55
        } else if (c >= 97 && c <= 102) { // 'a' to 'f'
            c - 87
        } else {
            abort 1
        }
    }

    // Convert a base16 encoded string to a vector of u8
    public(friend) fun base16_utf8_to_vec_u8(str: vector<u8>): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < vector::length(&str)) {
            let c1 = vector::borrow(&str, i);
            let c2 = vector::borrow(&str, i + 1);
            let byte = hex_char_to_u8(*c1) << 4 | hex_char_to_u8(*c2);
            vector::push_back(&mut result, byte);
            i = i + 2;
        };
        result
    }

}
