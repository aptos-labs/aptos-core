module poc::empty {
   use std::vector;

   public entry fun main(_owner: &signer) {
      let v = vector::empty<u64>();
      vector::destroy_empty(v);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
