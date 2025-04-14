module poc::internal_sub_string {
   use std::string::{Self};

   public entry fun main(_owner: &signer) {
      let s = string::utf8(b"hello world");
      let sub = string::sub_string(&s, 6, 11);
      let _ = sub;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
