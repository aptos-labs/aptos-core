module poc::internal_is_char_boundary {
   use std::string::{Self};

   public entry fun main(_owner:&signer) {
      let s1 = string::utf8(b"hello world");
      let s2 = string::utf8(b"hello world");
      string::insert(&mut s1, 6, s2);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
