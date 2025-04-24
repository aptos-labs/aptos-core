module poc::internal_check_utf8 {
   use std::string;


   public entry fun main(_owner: &signer) {
      let data = vector[0xF0u8, 0x9F , 0x98, 0x80];
      let _is_utf8 = string::internal_check_utf8(&data);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
