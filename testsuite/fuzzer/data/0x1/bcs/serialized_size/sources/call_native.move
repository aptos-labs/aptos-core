module poc::serialized_size {
   use std::bcs;

   public entry fun main(_owner: &signer) {
      let x: u64 = 42;
      let _size = bcs::serialized_size(&x);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
