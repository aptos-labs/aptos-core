module poc::constant_serialized_size {
   use std::bcs;

   public entry fun main(_owner: &signer) {
      let _size = bcs::constant_serialized_size<u64>();
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
