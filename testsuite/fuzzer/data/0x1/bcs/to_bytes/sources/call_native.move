module poc::to_bytes {
   use std::bcs;

   public entry fun main(_owner: &signer) {
      let x: u64 = 42;
      let _bytes = bcs::to_bytes(&x);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
