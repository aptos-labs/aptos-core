module poc::compare {
   use std::cmp;

   public entry fun main(_owner: &signer) {
      let a = vector[1u8, 2u8, 3u8];
      let b = vector[1u8, 2u8, 4u8];
      let _result = cmp::compare(&a, &b);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
