spec aptos_framework::base16 {

  spec hex_char_to_u8(c: u8): u8 {
      aborts_if !((c >= 48 && c <= 57) || (c >= 65 && c <= 70) || (c >= 97 && c <= 102));
      ensures result < 16;
      ensures result == (if (c <= 57) { c - 48 } else if (c <= 70) { c - 55 } else { c - 87 });
  }

  spec base16_utf8_to_vec_u8(str: vector<u8>): vector<u8> {
      pragma opaque;
      ensures [abstract] result == spec_base16_utf8_to_vec_u8(str);
  }

  spec fun spec_base16_utf8_to_vec_u8(str: vector<u8>): vector<u8>;
}
