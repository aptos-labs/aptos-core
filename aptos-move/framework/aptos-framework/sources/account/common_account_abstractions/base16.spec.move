spec aptos_framework::base16 {

  spec base16_utf8_to_vec_u8(str: vector<u8>): vector<u8> {
      pragma opaque;
      ensures [abstract] result == spec_base16_utf8_to_vec_u8(str);
  }

  spec fun spec_base16_utf8_to_vec_u8(str: vector<u8>): vector<u8>;
}
