module poc::borrow {
   use std::vector;

   public entry fun main(_owner: &signer) {
      let v = vector[1u64, 2u64, 3u64];
      let elem_ref = vector::borrow(&v, 1);
      let _ = elem_ref;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
