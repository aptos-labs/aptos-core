module poc::internal_index_of {
   use std::string::{Self};

   public entry fun main(_owner: &signer) {
      let s = string::utf8(b"hello world");
      let r = string::utf8(b"world");
      let idx = string::index_of(&s, &r);
      let _ = idx;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
